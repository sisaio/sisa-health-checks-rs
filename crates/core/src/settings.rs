//! The registry's settings, which a config file can hold.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::HealthError;

/// The longest any duration setting may be: one hour.
const MAX_MS: u64 = 60 * 60 * 1000;

/// How often the registry runs its checks, and how long each may take by default.
///
/// In a config file, missing fields keep their defaults and an unknown field is an error, so a
/// misspelled setting can't silently keep its default:
///
/// ```
/// use sisa_health_checks_core::HealthSettings;
///
/// let settings: HealthSettings = serde_json::from_str(r#"{ "poll_interval_ms": 10000 }"#)?;
///
/// assert_eq!(settings.poll_interval_ms, 10_000);
/// assert_eq!(settings.default_timeout_ms, 2_000);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct HealthSettings {
    /// How often the poller runs every check: 5 seconds by default. Probes read the latest
    /// results, so probe traffic doesn't add check runs.
    pub poll_interval_ms: u64,

    /// How long a check may run unless it sets its own timeout: 2 seconds by default. Keep it
    /// under the probe's `timeoutSeconds`.
    pub default_timeout_ms: u64,
}

// Every 5 seconds, 2 seconds per check.
impl Default for HealthSettings {
    fn default() -> Self {
        Self {
            poll_interval_ms: 5_000,
            default_timeout_ms: 2_000,
        }
    }
}

/// Fails when a duration setting is 0 or over an hour.
fn check_ms(field: &'static str, value: u64) -> Result<(), HealthError> {
    if value == 0 || value > MAX_MS {
        return Err(HealthError::InvalidSettings {
            field,
            reason: format!("is {value} ms; use a value from 1 ms to 1 hour"),
        });
    }

    Ok(())
}

impl HealthSettings {
    /// Fails when `poll_interval_ms` or `default_timeout_ms` is 0 or over an hour.
    pub fn validate(&self) -> Result<(), HealthError> {
        check_ms("poll_interval_ms", self.poll_interval_ms)?;
        check_ms("default_timeout_ms", self.default_timeout_ms)
    }

    /// `poll_interval_ms` as a duration.
    pub fn poll_interval(&self) -> Duration {
        Duration::from_millis(self.poll_interval_ms)
    }

    /// `default_timeout_ms` as a duration.
    pub fn default_timeout(&self) -> Duration {
        Duration::from_millis(self.default_timeout_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_valid() {
        assert_eq!(HealthSettings::default().validate(), Ok(()));
    }

    #[test]
    fn a_zero_interval_is_rejected() {
        let settings = HealthSettings {
            poll_interval_ms: 0,
            ..HealthSettings::default()
        };

        assert!(matches!(
            settings.validate(),
            Err(HealthError::InvalidSettings {
                field: "poll_interval_ms",
                ..
            })
        ));
    }

    #[test]
    fn an_unknown_field_is_rejected() {
        let parsed = serde_json::from_str::<HealthSettings>(r#"{ "poll_interval": 1000 }"#);

        assert!(parsed.is_err());
    }
}
