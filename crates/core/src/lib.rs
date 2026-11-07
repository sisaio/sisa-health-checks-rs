//! The shared parts of Sisa Health Checks: the [`HealthCheck`] contract, the [`HealthRegistry`]
//! that runs checks in the background, and the probe answers and report the HTTP crate serves.
//!
//! Applications depend on `sisa-health-checks`, which re-exports this crate's types, or on this
//! crate and the crates of the checks and the framework they use.
//!
//! A registry holds named checks. Each check is tagged with the probes that run it: [`LIVE`] for
//! `/livez`, which asks whether the process should be restarted and so checks only the process
//! itself, and [`READY`] for `/readyz`, which asks whether the pod should get traffic and so
//! checks the dependencies it can't serve without. A check is tagged [`READY`] unless given other
//! tags.
//!
//! The poller runs every check side by side on an interval, each under a timeout, and keeps the
//! results. Probes and the report read the latest results and never run a check, so the cost of
//! checking doesn't grow with probe traffic.
//!
//! ```
//! use std::time::Duration;
//!
//! use sisa_health_checks_core::{
//!     EventLoopCheck, HealthRegistry, HealthResult, LIVE, Probe, Status,
//! };
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), sisa_health_checks_core::HealthError> {
//! let registry = HealthRegistry::builder()
//!     .check("event-loop", EventLoopCheck::default())
//!     .tags([LIVE])
//!     .check_fn("database", async || HealthResult::healthy())
//!     .check_fn("search", async || HealthResult::unhealthy("index is rebuilding"))
//!     .failure(Status::Degraded)
//!     .build()?;
//!
//! // Runs every check now and every 5 seconds, until `poller` drops.
//! let poller = registry.spawn();
//! # registry.run().await;
//!
//! assert!(registry.probe(Probe::Live).passed());
//! assert!(registry.probe(Probe::Ready).passed());
//!
//! // On SIGTERM: fail `/readyz` first, so traffic moves away before the server stops.
//! registry.begin_draining();
//! assert!(!registry.probe(Probe::Ready).passed());
//! # drop(poller);
//! # Ok(())
//! # }
//! ```
//!
//! Checks log through [`tracing`](https://docs.rs/tracing) when their status changes, and record
//! their status and duration through the OpenTelemetry metrics API, as the [`metrics`] module
//! lists.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]

mod check;
mod error;
pub mod metrics;
mod registry;
mod report;
mod result;
mod runtime;
mod settings;
mod status;

pub use check::{
    CheckContext, FnCheck, HealthCheck, LIVE, NamedCheck, ProvidesHealthChecks, READY, check_fn,
};
pub use error::HealthError;
pub use registry::{HealthRegistry, HealthRegistryBuilder, PollerHandle};
pub use report::{CheckReport, HealthReport, Probe, ProbeLine, ProbeReport};
pub use result::HealthResult;
pub use runtime::EventLoopCheck;
pub use settings::HealthSettings;
pub use status::Status;
