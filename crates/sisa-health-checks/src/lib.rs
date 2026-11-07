//! Kubernetes-style health checks for Rust services: a registry that runs checks in the
//! background, `/livez` and `/readyz` probes that read the latest results, and an
//! `application/health+json` report for people and dashboards.
//!
//! No framework or check is on by default; turn on the ones you use:
//!
//! | Feature | Adds |
//! | --- | --- |
//! | `axum` | `probe_routes` and `report_routes`: `/livez`, `/readyz` and `/health` |
//! | `postgres` | `PostgresCheck`, on an sqlx pool, and the `sqlx` crate it is built on |
//! | `redis` | `RedisCheck`, on any redis-rs async connection, and the `redis` crate |
//! | `nats` | `NatsCheck`, on an async-nats client, and the `async_nats` crate |
//!
//! A check is tagged with the probes that run it: `live` for `/livez`, which must only check the
//! process itself, and `ready` for `/readyz`, which checks the dependencies the pod can't serve
//! without. Checks are tagged `ready` unless given other tags. A check of an optional dependency
//! fails as `Degraded`, so it is reported without taking the pod out of rotation.
//!
//! ```
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), sisa_health_checks::HealthError> {
//! use sisa_health_checks::{EventLoopCheck, HealthRegistry, HealthResult, LIVE, Probe, Status};
//!
//! let registry = HealthRegistry::builder()
//!     .check("event-loop", EventLoopCheck::default())
//!     .tags([LIVE])
//!     .check_fn("cache", async || HealthResult::unhealthy("connection refused"))
//!     .failure(Status::Degraded)
//!     .build()?;
//!
//! // Runs every check now and every 5 seconds, until `poller` drops.
//! let poller = registry.spawn();
//! # registry.run().await;
//!
//! assert!(registry.probe(Probe::Ready).passed());
//! # drop(poller);
//! # Ok(())
//! # }
//! ```
//!
//! Your own checks implement [`HealthCheck`] with an `async fn`, or come from a closure through
//! [`check_fn`]. Each part is also its own crate (`sisa-health-checks-core`, `-axum`,
//! `-postgres`, `-redis`, `-nats`), whose docs cover the details.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]

pub use sisa_health_checks_core::{
    CheckContext, CheckReport, EventLoopCheck, FnCheck, HealthCheck, HealthError, HealthRegistry,
    HealthRegistryBuilder, HealthReport, HealthResult, HealthSettings, LIVE, NamedCheck,
    PollerHandle, Probe, ProbeLine, ProbeReport, ProvidesHealthChecks, READY, Status, check_fn,
    metrics,
};

#[cfg(feature = "axum")]
pub use sisa_health_checks_axum::{
    LIVE_PATH, READY_PATH, REPORT_PATH, probe_routes, report_routes,
};

#[cfg(feature = "postgres")]
pub use sisa_health_checks_postgres::{PostgresCheck, sqlx};

#[cfg(feature = "redis")]
pub use sisa_health_checks_redis::{RedisCheck, redis};

#[cfg(feature = "nats")]
pub use sisa_health_checks_nats::{NatsCheck, async_nats};
