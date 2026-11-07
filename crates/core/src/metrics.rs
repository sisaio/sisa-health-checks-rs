//! The metrics the registry records, through the [OpenTelemetry](https://opentelemetry.io)
//! metrics API under the meter `sisa-health-checks`. Nothing is recorded until the application
//! installs an OpenTelemetry SDK meter provider, for example one that pushes over OTLP. Install
//! it before the first poll: the instruments are created then, from the provider installed at
//! that time.
//!
//! | Metric | Type | What it records |
//! | --- | --- | --- |
//! | `sisa.health.status` | gauge | Each check's latest status: 1 healthy, 0.5 degraded, 0 unhealthy |
//! | `sisa.health.check.duration` | histogram, seconds | How long each check run took, with `status` `healthy`, `degraded` or `unhealthy` |
//!
//! Both have a `check` attribute with the check's name. Names are a fixed set per registry, so
//! the series stay bounded. A run that timed out records the full timeout.
//!
//! Alert on the gauge for history across pods, and leave the probes to Kubernetes: a probe's
//! answer reaches only the kubelet, while the gauge reaches dashboards.

use std::{
    sync::{Arc, LazyLock},
    time::Duration,
};

use opentelemetry::{
    KeyValue, Value, global,
    metrics::{Gauge, Histogram},
};

use crate::status::Status;

/// The instruments, each recorded through one method so the attribute names are written once.
#[doc(hidden)]
#[derive(Debug)]
pub struct HealthMetrics {
    status: Gauge<f64>,
    duration: Histogram<f64>,
}

impl HealthMetrics {
    /// Records one run of the check `name`: its status and how long it took.
    pub(crate) fn record(&self, name: &Arc<str>, status: Status, duration: Duration) {
        let check = KeyValue::new("check", Value::String(name.clone().into()));

        self.status.record(status.gauge_value(), &[check.clone()]);
        self.duration.record(
            duration.as_secs_f64(),
            &[check, KeyValue::new("status", status.as_str())],
        );
    }
}

/// The instruments every registry records into.
#[doc(hidden)]
pub static METRICS: LazyLock<HealthMetrics> = LazyLock::new(|| {
    let meter = global::meter("sisa-health-checks");

    HealthMetrics {
        status: meter
            .f64_gauge("sisa.health.status")
            .with_description(
                "Latest status of each health check: 1 healthy, 0.5 degraded, 0 unhealthy",
            )
            .build(),
        duration: meter
            .f64_histogram("sisa.health.check.duration")
            .with_unit("s")
            .with_description("How long each health check run took")
            .with_boundaries(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ])
            .build(),
    }
});
