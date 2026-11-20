//! The log lines a registry writes. This test has a binary of its own: it installs the process's
//! one global subscriber, which would count the lines of any other test too.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use sisa_health_checks_core::{HealthRegistry, HealthResult};
use tracing::{
    Event, Level, Metadata, Subscriber,
    span::{Attributes, Id, Record},
};

/// How many warnings and info lines were logged. Clones share the counts.
#[derive(Clone, Default)]
struct Logged {
    warnings: Arc<AtomicUsize>,
    infos: Arc<AtomicUsize>,
}

impl Logged {
    /// The warnings, then the info lines.
    fn counts(&self) -> (usize, usize) {
        (
            self.warnings.load(Ordering::Relaxed),
            self.infos.load(Ordering::Relaxed),
        )
    }
}

// Counts the log lines, and lets every span through unrecorded.
impl Subscriber for Logged {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _attributes: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        match *event.metadata().level() {
            Level::WARN => self.warnings.fetch_add(1, Ordering::Relaxed),
            Level::INFO => self.infos.fetch_add(1, Ordering::Relaxed),
            _ => 0,
        };
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

#[tokio::test]
async fn a_status_logs_when_it_changes_not_at_every_run() {
    let logged = Logged::default();

    tracing::subscriber::set_global_default(logged.clone())
        .expect("no other subscriber is installed in this binary");

    let down = Arc::new(AtomicBool::new(true));
    let dependency = down.clone();
    let health = HealthRegistry::builder()
        .check_fn("nats", move || {
            let is_down = dependency.load(Ordering::Relaxed);

            async move {
                if is_down {
                    HealthResult::unhealthy("disconnected")
                } else {
                    HealthResult::healthy()
                }
            }
        })
        .build()
        .expect("the test builds a valid registry");

    health.run().await;
    health.run().await;
    assert_eq!(logged.counts(), (1, 0));

    down.store(false, Ordering::Relaxed);
    health.run().await;
    health.run().await;
    assert_eq!(logged.counts(), (1, 1));
}
