//! The contract a health check implements, a check made from a closure, and a check with its
//! registration options.

use std::{fmt, future::Future, pin::Pin, sync::Arc, time::Duration};

use crate::{result::HealthResult, status::Status};

/// The tag of checks that `/livez` runs: whether the process should be restarted. Only checks of
/// the process itself belong here, never a database or a broker.
pub const LIVE: &str = "live";

/// The tag of checks that `/readyz` runs: whether the pod should get traffic. Every check has it
/// unless it is given other tags.
pub const READY: &str = "ready";

/// What a check is told when it runs: its name and how long it has.
#[derive(Debug, Clone)]
pub struct CheckContext {
    name: Arc<str>,
    timeout: Duration,
}

impl CheckContext {
    pub(crate) fn new(name: Arc<str>, timeout: Duration) -> Self {
        Self { name, timeout }
    }

    /// The name the check was registered under.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// How long the check may run before the registry counts it as failed. A check needs no timer
    /// of its own; it can use this to bound an inner wait shorter than the whole run.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

/// A check of one dependency or of the process itself.
///
/// The registry runs it in the background on every poll, with a timeout, and catches a panic, so
/// `check` needs neither. Keep it cheap: one small query or command. Implement it with an
/// `async fn`:
///
/// ```
/// use sisa_health_checks_core::{CheckContext, HealthCheck, HealthResult};
///
/// struct DiskSpace {
///     min_free_percent: u8,
/// }
///
/// impl HealthCheck for DiskSpace {
///     async fn check(&self, _context: &CheckContext) -> HealthResult {
///         let free_percent = 42; // Read it from the system.
///
///         if free_percent < self.min_free_percent {
///             HealthResult::degraded("disk almost full").with_detail("free_percent", free_percent)
///         } else {
///             HealthResult::healthy().with_detail("free_percent", free_percent)
///         }
///     }
/// }
/// ```
pub trait HealthCheck: Send + Sync + 'static {
    /// Checks once and says what it found.
    fn check(&self, context: &CheckContext) -> impl Future<Output = HealthResult> + Send;
}

/// A boxed check future, so checks of different types sit in one list.
type BoxedCheck<'a> = Pin<Box<dyn Future<Output = HealthResult> + Send + 'a>>;

/// [`HealthCheck`] behind a pointer: its `impl Future` return can't be called through `dyn`.
pub(crate) trait DynCheck: Send + Sync {
    fn check_boxed<'a>(&'a self, context: &'a CheckContext) -> BoxedCheck<'a>;
}

// Boxes each check's future, so the registry holds every kind of check in one list.
impl<C: HealthCheck> DynCheck for C {
    fn check_boxed<'a>(&'a self, context: &'a CheckContext) -> BoxedCheck<'a> {
        Box::pin(self.check(context))
    }
}

/// A check made from a closure, returned by [`check_fn`].
pub struct FnCheck<F> {
    function: F,
}

// Names the type only: a closure has nothing to show.
impl<F> fmt::Debug for FnCheck<F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("FnCheck").finish_non_exhaustive()
    }
}

// Runs the closure on every poll.
impl<F, Fut> HealthCheck for FnCheck<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = HealthResult> + Send,
{
    fn check(&self, _context: &CheckContext) -> impl Future<Output = HealthResult> + Send {
        (self.function)()
    }
}

/// A check made from a closure, for one that needs no type of its own:
///
/// ```
/// use sisa_health_checks_core::{HealthResult, check_fn};
///
/// let license = check_fn(async || HealthResult::healthy());
/// # let _ = license;
/// ```
pub fn check_fn<F, Fut>(function: F) -> FnCheck<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = HealthResult> + Send,
{
    FnCheck { function }
}

/// A check with the options it is registered with: its name, tags, timeout and the worst status
/// it may report.
///
/// [`HealthRegistryBuilder`](crate::HealthRegistryBuilder) builds these for its own `check` calls.
/// A crate builds them to hand its checks to an application through [`ProvidesHealthChecks`].
pub struct NamedCheck {
    pub(crate) name: String,
    pub(crate) tags: Vec<String>,
    pub(crate) timeout: Option<Duration>,
    pub(crate) failure: Status,
    pub(crate) check: Box<dyn DynCheck>,
}

impl NamedCheck {
    /// `check` under `name`, tagged [`READY`], with the registry's default timeout, failing as
    /// [`Status::Unhealthy`].
    ///
    /// A name is ASCII letters, digits, `-`, `_` and `.`, unique in its registry. `startup`,
    /// `shutdown` and `poller` are taken by the registry's own probe lines.
    pub fn new(name: impl Into<String>, check: impl HealthCheck) -> Self {
        Self {
            name: name.into(),
            tags: vec![READY.to_string()],
            timeout: None,
            failure: Status::Unhealthy,
            check: Box::new(check),
        }
    }

    /// Replaces the tags, which pick the probes that run the check: [`LIVE`] for `/livez`,
    /// [`READY`] for `/readyz`. Other tags are kept for the report.
    pub fn tags<I, T>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    /// Takes the check out of every probe, so only the report shows it: for a dependency worth
    /// watching that the pod can serve without.
    pub fn report_only(mut self) -> Self {
        self.tags.clear();
        self
    }

    /// How long the check may run before it counts as failed, instead of the registry's default.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// The worst status the check reports: [`Status::Degraded`] for an optional dependency, so a
    /// failure is reported but the probe still passes. `build` rejects [`Status::Healthy`].
    pub fn failure(mut self, status: Status) -> Self {
        self.failure = status;
        self
    }

    /// The name the check is registered under.
    pub fn name(&self) -> &str {
        &self.name
    }
}

// Shows the registration options; the check itself has nothing to show.
impl fmt::Debug for NamedCheck {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NamedCheck")
            .field("name", &self.name)
            .field("tags", &self.tags)
            .field("timeout", &self.timeout)
            .field("failure", &self.failure)
            .finish_non_exhaustive()
    }
}

/// A type that ships its own checks, such as a client crate, so adding it to an application adds
/// them: `HealthRegistry::builder().extend(component.health_checks())`.
pub trait ProvidesHealthChecks {
    /// The checks, with the names, tags and timeouts they should have.
    fn health_checks(&self) -> Vec<NamedCheck>;
}
