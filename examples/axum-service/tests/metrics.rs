//! The registry's metrics, read back through `sisa_telemetry::testing::capture`, which installs
//! the providers `sisa_telemetry::init` does with in-memory exporters. This test has a binary of
//! its own: `capture` installs them for the whole process, and the instruments are made once per
//! process, from the meter provider installed at the first poll.

use sisa_health_checks::{HealthRegistry, HealthResult, Status};
use sisa_telemetry::testing::{self, CapturedMetric};

/// Each point as `key=value,... value`, in order. A histogram's value is how many runs it
/// counted.
fn points(metric: &CapturedMetric) -> Vec<String> {
    let mut points: Vec<String> = metric
        .points
        .iter()
        .map(|point| {
            let mut attributes: Vec<String> = point
                .attributes
                .iter()
                .map(|attribute| format!("{}={}", attribute.key, attribute.value))
                .collect();

            attributes.sort();

            let value = if point.bounds.is_empty() {
                point.value.to_string()
            } else {
                point.count.to_string()
            };

            format!("{} {value}", attributes.join(","))
        })
        .collect();

    points.sort();
    points
}

#[tokio::test]
async fn a_run_records_into_the_installed_telemetry() {
    // Before the registry, as `main` calls `init`: the first poll makes the instruments.
    let captured = testing::capture();

    let registry = HealthRegistry::builder()
        .check_fn("postgres", async || HealthResult::healthy())
        .check_fn("redis", async || HealthResult::unhealthy("down"))
        .failure(Status::Degraded)
        .build()
        .expect("the test builds a valid registry");

    registry.run().await;

    let status = captured
        .metric("sisa.health.status")
        .expect("a run records each check's status");
    let duration = captured
        .metric("sisa.health.check.duration")
        .expect("a run records each check's duration");

    assert_eq!(points(&status), ["check=postgres 1", "check=redis 0.5"]);
    assert_eq!(duration.unit, "s");
    assert_eq!(
        points(&duration),
        [
            "check=postgres,status=healthy 1",
            "check=redis,status=degraded 1",
        ]
    );
}
