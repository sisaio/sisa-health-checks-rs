//! The registry: the checks, the poller that runs them, and the probes and report that read their
//! latest results.

use std::{
    panic::AssertUnwindSafe,
    sync::{
        Arc, PoisonError, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use futures_util::{FutureExt, future::join_all};
use tokio::{
    task::JoinHandle,
    time::{Instant, MissedTickBehavior, interval, timeout},
};
use tracing::Instrument;

use crate::{
    check::{CheckContext, DynCheck, HealthCheck, NamedCheck, check_fn},
    error::HealthError,
    metrics::METRICS,
    report::{CheckReport, HealthReport, Probe, ProbeLine, ProbeReport},
    result::HealthResult,
    settings::HealthSettings,
    status::Status,
};

/// The names of the registry's own probe lines, which no check may take.
const RESERVED_NAMES: [&str; 3] = ["startup", "shutdown", "poller"];

/// How many poll intervals may pass without new results before the `poller` line fails.
const STALE_INTERVALS: u32 = 3;

/// Whether `name` is non-empty ASCII letters, digits, `-`, `_` and `.`, and not reserved.
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && !RESERVED_NAMES.contains(&name)
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

/// A check with its options settled at build time.
struct Registered {
    name: Arc<str>,
    tags: Vec<String>,
    timeout: Duration,
    failure: Status,
    check: Box<dyn DynCheck>,
}

impl Registered {
    fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|own| own == tag)
    }
}

/// One finished run of a check.
#[derive(Debug, Clone)]
struct Outcome {
    result: HealthResult,
    duration: Duration,
}

/// The latest run of every check, in the order the checks were added, and when it finished.
struct Latest {
    outcomes: Vec<Option<Outcome>>,
    completed_at: Instant,
}

struct Inner {
    checks: Vec<Registered>,
    poll_interval: Duration,
    stale_after: Duration,
    wait_for_start: bool,
    started: AtomicBool,
    draining: AtomicBool,
    latest: RwLock<Latest>,
}

/// Runs one check under its timeout, catching a panic, and caps the result at the check's
/// failure status.
async fn run_check(registered: &Registered) -> Outcome {
    let context = CheckContext::new(registered.name.clone(), registered.timeout);
    let started = Instant::now();
    let span = tracing::debug_span!("health check", check = %registered.name);
    let checking = AssertUnwindSafe(registered.check.check_boxed(&context)).catch_unwind();
    let checked = timeout(registered.timeout, checking).instrument(span).await;
    let duration = started.elapsed();

    let result = match checked {
        Ok(Ok(result)) => result,
        Ok(Err(_panic)) => HealthResult::unhealthy("the check panicked"),
        Err(_elapsed) => HealthResult::unhealthy(format!(
            "timed out after {} ms",
            registered.timeout.as_millis()
        )),
    };

    Outcome {
        result: result.capped(registered.failure),
        duration,
    }
}

/// Logs a check's change of status: a warning when it gets worse or starts out unhealthy or
/// degraded, an info line when it gets better. A steady status logs nothing.
fn log_change(name: &str, previous: Option<Status>, outcome: &Outcome) {
    let status = outcome.result.status();
    let unchanged = previous == Some(status) || (previous.is_none() && status == Status::Healthy);

    if unchanged {
        return;
    }

    let description = outcome.result.description().unwrap_or_default();

    if previous.is_some_and(|before| status < before) {
        tracing::info!(check = name, %status, description, "health check improved");
    } else {
        let error = outcome.result.error().unwrap_or_default();

        tracing::warn!(check = name, %status, description, error, "health check got worse");
    }
}

/// The probe line of a check that has run.
fn checked_line(name: &str, result: &HealthResult) -> ProbeLine {
    let description = result.description().unwrap_or_default();

    match result.status() {
        Status::Healthy => ProbeLine::new(name, true, "ok"),
        Status::Degraded => ProbeLine::new(name, true, format!("degraded: {description}")),
        Status::Unhealthy => ProbeLine::new(name, false, format!("failed: {description}")),
    }
}

/// Builds a [`HealthRegistry`]: add checks, then set the options of the one added last.
///
/// `tags`, `report_only`, `timeout` and `failure` apply to the check added just before them.
/// Called before any check, they make [`build`](Self::build) fail with
/// [`HealthError::NoCheck`].
#[derive(Debug, Default)]
pub struct HealthRegistryBuilder {
    settings: HealthSettings,
    checks: Vec<NamedCheck>,
    wait_for_start: bool,
    misuse: Option<HealthError>,
}

impl HealthRegistryBuilder {
    /// Replaces the settings: the poll interval and the default timeout.
    pub fn settings(mut self, settings: HealthSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Adds `check` under `name`, tagged [`READY`](crate::READY), with the default timeout.
    pub fn check(mut self, name: impl Into<String>, check: impl HealthCheck) -> Self {
        self.checks.push(NamedCheck::new(name, check));
        self
    }

    /// Adds a check made from a closure, as [`check_fn`] does.
    pub fn check_fn<F, Fut>(self, name: impl Into<String>, function: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HealthResult> + Send,
    {
        self.check(name, check_fn(function))
    }

    /// Adds a check with its options already set.
    pub fn add(mut self, check: NamedCheck) -> Self {
        self.checks.push(check);
        self
    }

    /// Adds checks with their options already set, such as a crate's
    /// [`ProvidesHealthChecks::health_checks`](crate::ProvidesHealthChecks::health_checks).
    pub fn extend(mut self, checks: impl IntoIterator<Item = NamedCheck>) -> Self {
        self.checks.extend(checks);
        self
    }

    /// Sets the options of the check added last, or notes the misuse for `build`.
    fn configure_last(
        mut self,
        option: &'static str,
        configure: impl FnOnce(NamedCheck) -> NamedCheck,
    ) -> Self {
        match self.checks.pop() {
            Some(check) => self.checks.push(configure(check)),
            None => {
                self.misuse.get_or_insert(HealthError::NoCheck(option));
            }
        }

        self
    }

    /// Replaces the tags of the check added last, as [`NamedCheck::tags`] does.
    pub fn tags<I, T>(self, tags: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        self.configure_last("tags", |check| check.tags(tags))
    }

    /// Takes the check added last out of every probe, as [`NamedCheck::report_only`] does.
    pub fn report_only(self) -> Self {
        self.configure_last("report_only", NamedCheck::report_only)
    }

    /// Sets the timeout of the check added last, as [`NamedCheck::timeout`] does.
    pub fn timeout(self, timeout: Duration) -> Self {
        self.configure_last("timeout", |check| check.timeout(timeout))
    }

    /// Sets the worst status the check added last reports, as [`NamedCheck::failure`] does.
    pub fn failure(self, status: Status) -> Self {
        self.configure_last("failure", |check| check.failure(status))
    }

    /// Makes `/readyz` fail until [`HealthRegistry::mark_started`] is called, for work the
    /// process must finish before it takes traffic, such as warming a cache. Without it,
    /// `/readyz` waits only for every check's first run.
    pub fn wait_for_start(mut self) -> Self {
        self.wait_for_start = true;
        self
    }

    /// The registry, or why it can't be built: invalid settings, a name that is invalid or taken
    /// twice, a zero or over-an-hour timeout, a `failure` of [`Status::Healthy`], or an option set
    /// before any check.
    pub fn build(self) -> Result<HealthRegistry, HealthError> {
        if let Some(misuse) = self.misuse {
            return Err(misuse);
        }

        self.settings.validate()?;

        let mut checks: Vec<Registered> = Vec::with_capacity(self.checks.len());

        for check in self.checks {
            if !valid_name(&check.name) {
                return Err(HealthError::InvalidName(check.name));
            }

            if checks.iter().any(|other| *other.name == *check.name) {
                return Err(HealthError::DuplicateName(check.name));
            }

            let timeout = check.timeout.unwrap_or(self.settings.default_timeout());

            if timeout.is_zero() || timeout > Duration::from_secs(60 * 60) {
                return Err(HealthError::InvalidCheck {
                    name: check.name,
                    reason: format!(
                        "has a timeout of {} ms; use a value from 1 ms to 1 hour",
                        timeout.as_millis()
                    ),
                });
            }

            if check.failure == Status::Healthy {
                return Err(HealthError::InvalidCheck {
                    name: check.name,
                    reason: "has failure Healthy, which would hide every failure; use Degraded or Unhealthy".into(),
                });
            }

            checks.push(Registered {
                name: Arc::from(check.name),
                tags: check.tags,
                timeout,
                failure: check.failure,
                check: check.check,
            });
        }

        let poll_interval = self.settings.poll_interval();
        let longest_timeout = checks
            .iter()
            .map(|check| check.timeout)
            .max()
            .unwrap_or_default();
        let outcomes = checks.iter().map(|_| None).collect();

        Ok(HealthRegistry {
            inner: Arc::new(Inner {
                checks,
                poll_interval,
                stale_after: poll_interval * STALE_INTERVALS + longest_timeout,
                wait_for_start: self.wait_for_start,
                started: AtomicBool::new(false),
                draining: AtomicBool::new(false),
                latest: RwLock::new(Latest {
                    outcomes,
                    completed_at: Instant::now(),
                }),
            }),
        })
    }
}

/// The checks of one process, their latest results, and the probes that read them.
///
/// The poller started by [`spawn`](Self::spawn) runs every check side by side on each poll
/// interval. Probes and the report read the latest results and never run a check, so probe
/// traffic costs no database queries. Cloning is cheap and shares everything.
///
/// ```
/// use std::time::Duration;
///
/// use sisa_health_checks_core::{HealthRegistry, HealthResult, Probe, Status};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), sisa_health_checks_core::HealthError> {
/// let registry = HealthRegistry::builder()
///     .check_fn("database", async || HealthResult::healthy())
///     .timeout(Duration::from_secs(2))
///     .check_fn("search", async || HealthResult::unhealthy("index is rebuilding"))
///     .failure(Status::Degraded)
///     .build()?;
///
/// // A service calls `spawn` instead and keeps the handle; one run is enough here.
/// registry.run().await;
///
/// let readiness = registry.probe(Probe::Ready);
///
/// assert!(readiness.passed());
/// assert_eq!(
///     readiness.to_text(true),
///     "[+]database ok\n[+]search degraded: index is rebuilding\n[+]shutdown ok\n[+]poller ok\nreadyz check passed\n"
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct HealthRegistry {
    inner: Arc<Inner>,
}

impl HealthRegistry {
    /// Starts building a registry.
    pub fn builder() -> HealthRegistryBuilder {
        HealthRegistryBuilder::default()
    }

    /// Runs every check once, side by side, each under its timeout, and keeps the results for
    /// the probes and the report. Records the metrics and logs each change of status.
    pub async fn run(&self) {
        let outcomes = join_all(self.inner.checks.iter().map(run_check)).await;
        let mut latest = self
            .inner
            .latest
            .write()
            .unwrap_or_else(PoisonError::into_inner);

        for ((registered, previous), outcome) in self
            .inner
            .checks
            .iter()
            .zip(&latest.outcomes)
            .zip(&outcomes)
        {
            let previous = previous.as_ref().map(|before| before.result.status());

            log_change(&registered.name, previous, outcome);
            METRICS.record(&registered.name, outcome.result.status(), outcome.duration);
        }

        latest.outcomes = outcomes.into_iter().map(Some).collect();
        latest.completed_at = Instant::now();
    }

    /// Starts the poller: a task that runs every check now and then once per poll interval, until
    /// the returned handle drops. Call it once, inside a Tokio runtime, and keep the handle for
    /// the life of the process.
    pub fn spawn(&self) -> PollerHandle {
        let registry = self.clone();

        let task = tokio::spawn(async move {
            let mut ticks = interval(registry.inner.poll_interval);

            // A slow run delays the next one instead of starting a burst to catch up.
            ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);

            loop {
                ticks.tick().await;
                registry.run().await;
            }
        });

        PollerHandle { task }
    }

    /// Lets `/readyz` pass, for a registry built with
    /// [`wait_for_start`](HealthRegistryBuilder::wait_for_start). Call it once the process has
    /// finished its startup work.
    pub fn mark_started(&self) {
        self.inner.started.store(true, Ordering::Relaxed);
    }

    /// Makes `/readyz` fail from now on, so load balancers stop sending traffic while the process
    /// shuts down. `/livez` is unaffected: a draining process is working as it should, and
    /// restarting it is the one thing that would hurt. Call it first on SIGTERM, wait a few
    /// seconds for the endpoints to update, then stop the server.
    pub fn begin_draining(&self) {
        self.inner.draining.store(true, Ordering::Relaxed);
    }

    /// Whether [`begin_draining`](Self::begin_draining) was called.
    pub fn is_draining(&self) -> bool {
        self.inner.draining.load(Ordering::Relaxed)
    }

    /// The `poller` line: failing once no run has finished for three poll intervals plus the
    /// longest timeout, which means the poller stopped or was never started.
    fn poller_line(&self, latest: &Latest) -> ProbeLine {
        let since = latest.completed_at.elapsed();

        if since > self.inner.stale_after {
            ProbeLine::new(
                "poller",
                false,
                format!(
                    "failed: no results for {} s; is the poller running?",
                    since.as_secs()
                ),
            )
        } else {
            ProbeLine::new("poller", true, "ok")
        }
    }

    /// The answer of `probe`, from the latest results.
    ///
    /// A check that hasn't run yet fails `/readyz` and passes `/livez`, so a starting pod gets no
    /// traffic and isn't restarted either. `/readyz` also fails while draining and, with
    /// [`wait_for_start`](HealthRegistryBuilder::wait_for_start), until started. Both fail once
    /// the results are stale.
    pub fn probe(&self, probe: Probe) -> ProbeReport {
        let latest = self
            .inner
            .latest
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        let mut lines = Vec::new();

        for (registered, outcome) in self.inner.checks.iter().zip(&latest.outcomes) {
            if !registered.has_tag(probe.tag()) {
                continue;
            }

            lines.push(match outcome {
                Some(outcome) => checked_line(&registered.name, &outcome.result),
                None => ProbeLine::new(&*registered.name, probe == Probe::Live, "pending"),
            });
        }

        if probe == Probe::Ready {
            if self.inner.wait_for_start {
                lines.push(if self.inner.started.load(Ordering::Relaxed) {
                    ProbeLine::new("startup", true, "ok")
                } else {
                    ProbeLine::new("startup", false, "failed: not started yet")
                });
            }

            lines.push(if self.is_draining() {
                ProbeLine::new("shutdown", false, "failed: draining")
            } else {
                ProbeLine::new("shutdown", true, "ok")
            });
        }

        lines.push(self.poller_line(&latest));

        ProbeReport::new(probe, lines)
    }

    /// Every check's latest result, for the `/health` report.
    pub fn report(&self) -> HealthReport {
        let latest = self
            .inner
            .latest
            .read()
            .unwrap_or_else(PoisonError::into_inner);

        let checks = self
            .inner
            .checks
            .iter()
            .zip(&latest.outcomes)
            .map(|(registered, outcome)| CheckReport {
                name: registered.name.to_string(),
                tags: registered.tags.clone(),
                status: outcome.as_ref().map(|ran| ran.result.status()),
                description: outcome
                    .as_ref()
                    .and_then(|ran| ran.result.description().map(str::to_string)),
                details: outcome
                    .as_ref()
                    .map(|ran| ran.result.details().clone())
                    .unwrap_or_default(),
                duration: outcome.as_ref().map(|ran| ran.duration),
            })
            .collect();

        HealthReport {
            checks,
            draining: self.is_draining(),
        }
    }
}

// Shows the checks' names and the probe state, never a result's error.
impl std::fmt::Debug for HealthRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<&str> = self.inner.checks.iter().map(|check| &*check.name).collect();

        formatter
            .debug_struct("HealthRegistry")
            .field("checks", &names)
            .field("poll_interval", &self.inner.poll_interval)
            .field("draining", &self.is_draining())
            .finish_non_exhaustive()
    }
}

/// The poller task: it stops when this handle drops, so keep it for the life of the process.
#[derive(Debug)]
#[must_use = "the poller stops when its handle drops"]
pub struct PollerHandle {
    task: JoinHandle<()>,
}

// Stops the poller with its handle, so a dropped registry leaves no task behind.
impl Drop for PollerHandle {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use super::*;
    use crate::{EventLoopCheck, LIVE};

    fn registry(builder: HealthRegistryBuilder) -> HealthRegistry {
        builder.build().expect("the test builds a valid registry")
    }

    #[tokio::test(start_paused = true)]
    async fn readiness_passes_when_every_check_is_healthy() {
        let health = registry(
            HealthRegistry::builder()
                .check_fn("postgres", async || HealthResult::healthy())
                .check_fn("redis", async || HealthResult::healthy()),
        );

        health.run().await;

        assert!(health.probe(Probe::Ready).passed());
    }

    #[tokio::test(start_paused = true)]
    async fn one_unhealthy_check_fails_readiness() {
        let health = registry(
            HealthRegistry::builder()
                .check_fn("postgres", async || HealthResult::healthy())
                .check_fn("nats", async || HealthResult::unhealthy("disconnected")),
        );

        health.run().await;

        let readiness = health.probe(Probe::Ready);

        assert!(!readiness.passed());
        assert_eq!(
            readiness.to_text(false),
            "[-]nats failed: disconnected\nreadyz check failed\n"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_degraded_check_keeps_readiness() {
        let health = registry(
            HealthRegistry::builder()
                .check_fn("search", async || HealthResult::degraded("rebuilding")),
        );

        health.run().await;

        assert!(health.probe(Probe::Ready).passed());
    }

    #[tokio::test(start_paused = true)]
    async fn failure_degraded_reports_an_unhealthy_check_as_degraded() {
        let health = registry(
            HealthRegistry::builder()
                .check_fn("redis", async || {
                    HealthResult::unhealthy("connection refused")
                })
                .failure(Status::Degraded),
        );

        health.run().await;

        assert!(health.probe(Probe::Ready).passed());
        assert_eq!(health.report().status(), Status::Degraded);
    }

    #[tokio::test(start_paused = true)]
    async fn a_check_past_its_timeout_fails() {
        let health = registry(
            HealthRegistry::builder()
                .check_fn("slow", async || {
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    HealthResult::healthy()
                })
                .timeout(Duration::from_millis(500)),
        );

        health.run().await;

        let line = &health.probe(Probe::Ready).lines()[0];

        assert!(!line.passed());
        assert_eq!(line.message(), "failed: timed out after 500 ms");
    }

    #[tokio::test(start_paused = true)]
    async fn a_panicking_check_fails_without_stopping_the_others() {
        let health = registry(
            HealthRegistry::builder()
                .check_fn("broken", async || -> HealthResult {
                    panic!("bug in a check")
                })
                .check_fn("postgres", async || HealthResult::healthy()),
        );

        health.run().await;

        let lines = health.probe(Probe::Ready).lines().to_vec();

        assert_eq!(lines[0].message(), "failed: the check panicked");
        assert!(lines[1].passed());
    }

    #[tokio::test(start_paused = true)]
    async fn before_the_first_run_readiness_fails_and_liveness_passes() {
        let health = registry(
            HealthRegistry::builder()
                .add(NamedCheck::new("event-loop", EventLoopCheck::default()).tags([LIVE]))
                .check_fn("postgres", async || HealthResult::healthy()),
        );

        assert!(health.probe(Probe::Live).passed());
        assert!(!health.probe(Probe::Ready).passed());
    }

    #[tokio::test(start_paused = true)]
    async fn draining_fails_readiness_but_not_liveness() {
        let health = registry(
            HealthRegistry::builder()
                .check_fn("event-loop", async || HealthResult::healthy())
                .tags([LIVE]),
        );

        health.run().await;
        health.begin_draining();

        assert!(health.probe(Probe::Live).passed());
        assert!(!health.probe(Probe::Ready).passed());
    }

    #[tokio::test(start_paused = true)]
    async fn wait_for_start_holds_readiness_until_started() {
        let health = registry(HealthRegistry::builder().wait_for_start());

        health.run().await;
        assert!(!health.probe(Probe::Ready).passed());

        health.mark_started();
        assert!(health.probe(Probe::Ready).passed());
    }

    #[tokio::test(start_paused = true)]
    async fn stale_results_fail_both_probes() {
        let health = registry(HealthRegistry::builder());

        health.run().await;
        // Default settings: 3 intervals of 5 s, plus no check timeout.
        tokio::time::advance(Duration::from_secs(16)).await;

        assert!(!health.probe(Probe::Live).passed());
        assert!(!health.probe(Probe::Ready).passed());
    }

    #[tokio::test(start_paused = true)]
    async fn the_poller_keeps_the_results_fresh() {
        let runs = Arc::new(AtomicUsize::new(0));
        let counted = runs.clone();
        let health = registry(HealthRegistry::builder().check_fn("counted", move || {
            counted.fetch_add(1, Ordering::Relaxed);
            async { HealthResult::healthy() }
        }));

        let _poller = health.spawn();
        tokio::time::sleep(Duration::from_secs(60)).await;

        assert!(runs.load(Ordering::Relaxed) >= 12);
        assert!(health.probe(Probe::Ready).passed());
    }

    #[tokio::test(start_paused = true)]
    async fn report_only_checks_stay_out_of_the_probes() {
        let health = registry(
            HealthRegistry::builder()
                .check_fn("crm-api", async || HealthResult::unhealthy("down"))
                .report_only(),
        );

        health.run().await;

        assert!(health.probe(Probe::Ready).passed());
        assert_eq!(health.report().status(), Status::Unhealthy);
    }

    #[test]
    fn an_option_before_any_check_is_an_error() {
        let built = HealthRegistry::builder()
            .timeout(Duration::from_secs(1))
            .build();

        assert_eq!(built.err(), Some(HealthError::NoCheck("timeout")));
    }

    #[test]
    fn names_must_be_unique() {
        let built = HealthRegistry::builder()
            .check_fn("postgres", async || HealthResult::healthy())
            .check_fn("postgres", async || HealthResult::healthy())
            .build();

        assert_eq!(
            built.err(),
            Some(HealthError::DuplicateName("postgres".into()))
        );
    }

    #[test]
    fn reserved_and_odd_names_are_rejected() {
        for name in ["shutdown", "", "my check", "[x]"] {
            let built = HealthRegistry::builder()
                .check_fn(name, async || HealthResult::healthy())
                .build();

            assert_eq!(built.err(), Some(HealthError::InvalidName(name.into())));
        }
    }
}
