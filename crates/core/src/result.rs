//! What one run of a check found.

use std::error::Error;

use serde_json::{Map, Value};

use crate::status::Status;

/// What one run of a check found: a [`Status`], an optional description, details for the report,
/// and the error behind a failure.
///
/// The description is short and safe to show: it appears in the probes' `?verbose` listing and in
/// the report. Details appear only in the report. The error is logged when the check changes
/// state and never sent in a response, so it may name hosts or carry a driver's message.
///
/// ```
/// use sisa_health_checks_core::{HealthResult, Status};
///
/// let result = HealthResult::degraded("replica lag")
///     .with_detail("lag_seconds", 12)
///     .with_detail("replica", "db-2");
///
/// assert_eq!(result.status(), Status::Degraded);
/// assert_eq!(result.description(), Some("replica lag"));
/// assert_eq!(result.details()["lag_seconds"], 12);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct HealthResult {
    status: Status,
    description: Option<String>,
    details: Map<String, Value>,
    error: Option<String>,
}

impl HealthResult {
    fn new(status: Status, description: Option<String>) -> Self {
        Self {
            status,
            description,
            details: Map::new(),
            error: None,
        }
    }

    /// The check found its dependency working.
    pub fn healthy() -> Self {
        Self::new(Status::Healthy, None)
    }

    /// The check found its dependency working with a problem, said in `description`.
    pub fn degraded(description: impl Into<String>) -> Self {
        Self::new(Status::Degraded, Some(description.into()))
    }

    /// The check found its dependency not working, said in `description`.
    pub fn unhealthy(description: impl Into<String>) -> Self {
        Self::new(Status::Unhealthy, Some(description.into()))
    }

    /// Adds a detail the report shows under the check, such as a latency or a pool size. A later
    /// detail with the same key replaces the earlier one. Never put a secret or a connection
    /// string here: anyone who can read the report sees it.
    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }

    /// Keeps the error behind a failure, with its sources, for the log line written when the check
    /// changes state. Responses never include it.
    pub fn with_error(mut self, error: &dyn Error) -> Self {
        let mut message = error.to_string();
        let mut source = error.source();

        while let Some(cause) = source {
            message.push_str(": ");
            message.push_str(&cause.to_string());
            source = cause.source();
        }

        self.error = Some(message);
        self
    }

    /// How healthy the check found its dependency.
    pub fn status(&self) -> Status {
        self.status
    }

    /// The short description, set by [`degraded`](Self::degraded) and
    /// [`unhealthy`](Self::unhealthy).
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The details added with [`with_detail`](Self::with_detail).
    pub fn details(&self) -> &Map<String, Value> {
        &self.details
    }

    /// The error added with [`with_error`](Self::with_error), with its sources.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Lowers an `Unhealthy` result to `failure`, the worst status its check may report.
    pub(crate) fn capped(mut self, failure: Status) -> Self {
        self.status = self.status.min(failure);
        self
    }
}

#[cfg(test)]
mod tests {
    use std::{fmt, io};

    use super::*;

    #[derive(Debug)]
    struct Outer(io::Error);

    // Says what failed, and hands back the I/O error as its source.
    impl fmt::Display for Outer {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("query failed")
        }
    }

    // Gives the I/O error as the cause, so the chain has two links.
    impl Error for Outer {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Some(&self.0)
        }
    }

    #[test]
    fn with_error_keeps_the_whole_chain() {
        let error = Outer(io::Error::other("connection reset"));
        let result = HealthResult::unhealthy("database down").with_error(&error);

        assert_eq!(result.error(), Some("query failed: connection reset"));
    }

    #[test]
    fn a_later_detail_replaces_an_earlier_one() {
        let result = HealthResult::healthy()
            .with_detail("idle", 1)
            .with_detail("idle", 2);

        assert_eq!(result.details()["idle"], 2);
    }

    #[test]
    fn capping_lowers_only_worse_statuses() {
        let unhealthy = HealthResult::unhealthy("down").capped(Status::Degraded);
        let healthy = HealthResult::healthy().capped(Status::Degraded);

        assert_eq!(unhealthy.status(), Status::Degraded);
        assert_eq!(healthy.status(), Status::Healthy);
    }
}
