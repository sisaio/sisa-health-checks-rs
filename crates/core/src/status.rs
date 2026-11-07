//! How healthy a check found its dependency.

use std::fmt;

use serde::{Deserialize, Serialize};

/// How healthy a check found what it checks, from best to worst, so `max` of several statuses is
/// the worst of them.
///
/// A probe passes unless one of its checks is `Unhealthy`: `Degraded` is reported, but keeps the
/// probe at 200, so an optional dependency that is down doesn't take the pod out of rotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Works as it should.
    Healthy,
    /// Works, with a problem worth reporting, such as an optional dependency that is down.
    Degraded,
    /// Doesn't work: a probe that includes this check fails.
    Unhealthy,
}

impl Status {
    /// The status as a word: `healthy`, `degraded` or `unhealthy`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
        }
    }

    /// Whether a probe that includes a check with this status passes: anything but `Unhealthy`.
    pub fn passes(self) -> bool {
        self != Self::Unhealthy
    }

    /// The status in the `application/health+json` draft's words: `pass`, `warn` or `fail`.
    pub fn as_health_json(self) -> &'static str {
        match self {
            Self::Healthy => "pass",
            Self::Degraded => "warn",
            Self::Unhealthy => "fail",
        }
    }

    /// The value of the `sisa.health.status` gauge: 1, 0.5 or 0.
    pub(crate) fn gauge_value(self) -> f64 {
        match self {
            Self::Healthy => 1.0,
            Self::Degraded => 0.5,
            Self::Unhealthy => 0.0,
        }
    }
}

// Shows the status as a word, as logs and probe lines print it.
impl fmt::Display for Status {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_worst_status_is_the_largest() {
        let statuses = [Status::Degraded, Status::Unhealthy, Status::Healthy];

        assert_eq!(statuses.into_iter().max(), Some(Status::Unhealthy));
    }

    #[test]
    fn only_unhealthy_fails_a_probe() {
        assert!(Status::Healthy.passes());
        assert!(Status::Degraded.passes());
        assert!(!Status::Unhealthy.passes());
    }

    #[test]
    fn health_json_words_follow_the_draft() {
        assert_eq!(Status::Healthy.as_health_json(), "pass");
        assert_eq!(Status::Degraded.as_health_json(), "warn");
        assert_eq!(Status::Unhealthy.as_health_json(), "fail");
    }
}
