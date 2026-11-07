//! A check of the process itself: whether the async runtime still runs tasks on time.

use std::{future::Future, time::Duration};

use tokio::time::{Instant, sleep};

use crate::{check::CheckContext, check::HealthCheck, result::HealthResult};

/// How long the check sleeps, to see how late it wakes.
const NAP: Duration = Duration::from_millis(10);

/// Whether the Tokio runtime wakes tasks on time: it sleeps a few milliseconds and measures how
/// late it wakes. Code that blocks a runtime thread, such as heavy computing or a blocking call
/// outside `spawn_blocking`, makes every task late, and this check unhealthy.
///
/// It checks the process only, so it belongs in the liveness probe:
///
/// ```
/// use std::time::Duration;
///
/// use sisa_health_checks_core::{EventLoopCheck, HealthRegistry, LIVE};
///
/// let registry = HealthRegistry::builder()
///     .check("event-loop", EventLoopCheck::new(Duration::from_millis(100)))
///     .tags([LIVE])
///     .build()?;
/// # let _ = registry;
/// # Ok::<(), sisa_health_checks_core::HealthError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventLoopCheck {
    max_lag: Duration,
}

impl EventLoopCheck {
    /// A check that fails once a task wakes more than `max_lag` late.
    pub fn new(max_lag: Duration) -> Self {
        Self { max_lag }
    }

    /// How late a task may wake before the check fails.
    pub fn max_lag(&self) -> Duration {
        self.max_lag
    }
}

// 100 ms, long enough to ignore a busy moment and short enough to catch a blocked thread.
impl Default for EventLoopCheck {
    fn default() -> Self {
        Self::new(Duration::from_millis(100))
    }
}

// Sleeps briefly and compares when it woke with when it asked to.
impl HealthCheck for EventLoopCheck {
    fn check(&self, _context: &CheckContext) -> impl Future<Output = HealthResult> + Send {
        let max_lag = self.max_lag;

        async move {
            let started = Instant::now();

            sleep(NAP).await;

            let lag = started.elapsed().saturating_sub(NAP);
            let lag_ms = lag.as_secs_f64() * 1000.0;

            if lag > max_lag {
                HealthResult::unhealthy(format!(
                    "tasks wake {lag_ms:.0} ms late, over {} ms",
                    max_lag.as_millis()
                ))
                .with_detail("lag_ms", lag_ms)
            } else {
                HealthResult::healthy().with_detail("lag_ms", lag_ms)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::status::Status;

    #[tokio::test(start_paused = true)]
    async fn an_idle_runtime_is_healthy() {
        let context = CheckContext::new(Arc::from("event-loop"), Duration::from_secs(1));
        let result = EventLoopCheck::default().check(&context).await;

        assert_eq!(result.status(), Status::Healthy);
    }
}
