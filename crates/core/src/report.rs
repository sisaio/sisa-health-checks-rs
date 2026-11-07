//! What the probes and the report say, from the registry's latest results.

use std::time::Duration;

use serde_json::{Map, Value, json};

use crate::{
    check::{LIVE, READY},
    status::Status,
};

/// One of the probes Kubernetes calls, as the Kubernetes API server names its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Probe {
    /// `/livez`: whether the process should be restarted. Runs the checks tagged [`LIVE`].
    Live,
    /// `/readyz`: whether the pod should get traffic. Runs the checks tagged [`READY`], and fails
    /// while the registry drains or before it is started.
    Ready,
}

impl Probe {
    /// The tag of the checks this probe runs.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Live => LIVE,
            Self::Ready => READY,
        }
    }

    /// The endpoint's name, `livez` or `readyz`, which ends its text answer.
    pub fn endpoint(self) -> &'static str {
        match self {
            Self::Live => "livez",
            Self::Ready => "readyz",
        }
    }
}

/// One line of a probe's answer: a check, or one of the registry's own conditions (`startup`,
/// `shutdown` and `poller`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeLine {
    name: String,
    passed: bool,
    message: String,
}

impl ProbeLine {
    pub(crate) fn new(name: impl Into<String>, passed: bool, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed,
            message: message.into(),
        }
    }

    /// The check's or the condition's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether this line lets the probe pass.
    pub fn passed(&self) -> bool {
        self.passed
    }

    /// What the line says after the name: `ok`, `degraded: ...`, `pending` or `failed: ...`.
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// A probe's answer: it passes when every line passes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeReport {
    probe: Probe,
    lines: Vec<ProbeLine>,
}

impl ProbeReport {
    pub(crate) fn new(probe: Probe, lines: Vec<ProbeLine>) -> Self {
        Self { probe, lines }
    }

    /// Which probe this answers.
    pub fn probe(&self) -> Probe {
        self.probe
    }

    /// Whether the probe passes: 200 when it does, 503 when it doesn't.
    pub fn passed(&self) -> bool {
        self.lines.iter().all(ProbeLine::passed)
    }

    /// The lines: the probe's checks in the order they were added, then the registry's own
    /// conditions.
    pub fn lines(&self) -> &[ProbeLine] {
        &self.lines
    }

    /// The answer as the Kubernetes API server writes it. A passing probe says `ok`; a failing one
    /// lists its failing lines. `verbose` lists every line, each marked `[+]` or `[-]`, and ends
    /// with `readyz check passed` or `readyz check failed`:
    ///
    /// ```text
    /// [+]postgres ok
    /// [-]nats failed: timed out after 2000 ms
    /// [+]shutdown ok
    /// [+]poller ok
    /// readyz check failed
    /// ```
    pub fn to_text(&self, verbose: bool) -> String {
        let passed = self.passed();

        if passed && !verbose {
            return "ok".to_string();
        }

        let mut text = String::new();

        for line in self.lines.iter().filter(|line| verbose || !line.passed) {
            let mark = if line.passed { '+' } else { '-' };

            text.push_str(&format!("[{mark}]{} {}\n", line.name, line.message));
        }

        let outcome = if passed { "passed" } else { "failed" };

        text.push_str(&format!("{} check {outcome}\n", self.probe.endpoint()));
        text
    }
}

/// One check in the report, with its latest result.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckReport {
    pub(crate) name: String,
    pub(crate) tags: Vec<String>,
    /// `None` until the check has run once.
    pub(crate) status: Option<Status>,
    pub(crate) description: Option<String>,
    pub(crate) details: Map<String, Value>,
    pub(crate) duration: Option<Duration>,
}

impl CheckReport {
    /// The check's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The check's tags.
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// The latest status, or `None` before the check's first run.
    pub fn status(&self) -> Option<Status> {
        self.status
    }

    /// The latest description.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The latest details.
    pub fn details(&self) -> &Map<String, Value> {
        &self.details
    }

    /// How long the latest run took.
    pub fn duration(&self) -> Option<Duration> {
        self.duration
    }

    /// The check as one entry of the report's `checks` object.
    fn to_json(&self) -> Value {
        let mut entry = Map::new();
        let status = self.status.unwrap_or(Status::Unhealthy);

        entry.insert("status".into(), status.as_health_json().into());

        if let Some(duration) = self.duration {
            entry.insert(
                "observedValue".into(),
                (duration.as_secs_f64() * 1000.0).into(),
            );
            entry.insert("observedUnit".into(), "ms".into());
        }

        match (&self.description, self.status) {
            (Some(description), _) => {
                entry.insert("output".into(), description.as_str().into());
            }
            (None, None) => {
                entry.insert("output".into(), "not checked yet".into());
            }
            (None, Some(_)) => {}
        }

        entry.insert("tags".into(), self.tags.clone().into());

        if !self.details.is_empty() {
            entry.insert("details".into(), Value::Object(self.details.clone()));
        }

        Value::Object(entry)
    }
}

/// Every check's latest result, for people and dashboards rather than Kubernetes, which reads
/// only the probes.
#[derive(Debug, Clone, PartialEq)]
pub struct HealthReport {
    pub(crate) checks: Vec<CheckReport>,
    pub(crate) draining: bool,
}

impl HealthReport {
    /// The checks, in the order they were added.
    pub fn checks(&self) -> &[CheckReport] {
        &self.checks
    }

    /// Whether the registry is draining, so `/readyz` fails whatever the checks say.
    pub fn draining(&self) -> bool {
        self.draining
    }

    /// The worst status of any check, with a check that hasn't run yet counted as unhealthy.
    pub fn status(&self) -> Status {
        self.checks
            .iter()
            .map(|check| check.status.unwrap_or(Status::Unhealthy))
            .max()
            .unwrap_or(Status::Healthy)
    }

    /// The HTTP status code for the report: 503 when [`status`](Self::status) is unhealthy, else
    /// 200, as the `application/health+json` draft asks.
    pub fn http_status_code(&self) -> u16 {
        if self.status().passes() { 200 } else { 503 }
    }

    /// The report in the shape of the IETF draft "Health Check Response Format for HTTP APIs"
    /// (`application/health+json`), which expired as a draft and never became an RFC, but is the
    /// shape most tools read:
    ///
    /// ```json
    /// {
    ///   "status": "warn",
    ///   "checks": {
    ///     "postgres": [{ "status": "pass", "observedValue": 3.1, "observedUnit": "ms", "tags": ["ready"] }],
    ///     "search": [{ "status": "warn", "output": "index is rebuilding", "tags": ["ready"] }]
    ///   }
    /// }
    /// ```
    ///
    /// While the registry drains, `notes` says so.
    pub fn to_json(&self) -> Value {
        let mut checks = Map::new();

        for check in &self.checks {
            checks.insert(check.name.clone(), json!([check.to_json()]));
        }

        let mut report = Map::new();

        report.insert("status".into(), self.status().as_health_json().into());
        report.insert("checks".into(), Value::Object(checks));

        if self.draining {
            report.insert("notes".into(), json!(["draining: /readyz fails"]));
        }

        Value::Object(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(lines: Vec<ProbeLine>) -> ProbeReport {
        ProbeReport::new(Probe::Ready, lines)
    }

    #[test]
    fn a_passing_probe_says_ok() {
        let probe = report(vec![ProbeLine::new("postgres", true, "ok")]);

        assert_eq!(probe.to_text(false), "ok");
    }

    #[test]
    fn verbose_lists_every_line() {
        let probe = report(vec![
            ProbeLine::new("postgres", true, "ok"),
            ProbeLine::new("nats", false, "failed: timed out after 2000 ms"),
        ]);

        assert_eq!(
            probe.to_text(true),
            "[+]postgres ok\n[-]nats failed: timed out after 2000 ms\nreadyz check failed\n"
        );
    }

    #[test]
    fn a_failing_probe_lists_only_its_failures() {
        let probe = report(vec![
            ProbeLine::new("postgres", true, "ok"),
            ProbeLine::new("nats", false, "failed: disconnected"),
        ]);

        assert_eq!(
            probe.to_text(false),
            "[-]nats failed: disconnected\nreadyz check failed\n"
        );
    }

    #[test]
    fn the_report_is_as_bad_as_its_worst_check() {
        let check = |name: &str, status| CheckReport {
            name: name.into(),
            tags: vec![READY.into()],
            status,
            description: None,
            details: Map::new(),
            duration: None,
        };
        let health = HealthReport {
            checks: vec![
                check("postgres", Some(Status::Healthy)),
                check("redis", None),
            ],
            draining: false,
        };

        assert_eq!(health.status(), Status::Unhealthy);
        assert_eq!(health.http_status_code(), 503);
        assert_eq!(health.to_json()["status"], "fail");
        assert_eq!(
            health.to_json()["checks"]["redis"][0]["output"],
            "not checked yet"
        );
    }
}
