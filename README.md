# sisa-health-checks-rs

[![CI](https://github.com/sisaio/sisa-health-checks-rs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sisaio/sisa-health-checks-rs/actions/workflows/ci.yml)
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/sisaio/sisa-health-checks-rs/badge)](https://scorecard.dev/viewer/?uri=github.com/sisaio/sisa-health-checks-rs)
[![crates.io](https://img.shields.io/crates/v/sisa-health-checks.svg)](https://crates.io/crates/sisa-health-checks)
[![docs.rs](https://img.shields.io/docsrs/sisa-health-checks)](https://docs.rs/sisa-health-checks)
[![MSRV](https://img.shields.io/badge/MSRV-1.89-blue)](https://github.com/sisaio/sisa-health-checks-rs/blob/main/Cargo.toml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/sisaio/sisa-health-checks-rs/blob/main/LICENSE)

Kubernetes-style health checks for Rust services: a registry that runs checks in the background,
`/livez` and `/readyz` probes that answer from the latest results, an `application/health+json`
report, and ready-made checks for PostgreSQL, Redis and NATS.

## What you get

| Feature | Adds |
| --- | --- |
| (always) | `HealthRegistry`, the `HealthCheck` trait, `check_fn` for closures, `EventLoopCheck`, OpenTelemetry metrics |
| `axum` | `probe_routes` (`/livez`, `/readyz`) and `report_routes` (`/health`) |
| `postgres` | `PostgresCheck`, `SELECT 1` on an sqlx pool |
| `redis` | `RedisCheck`, `PING` on any redis-rs async connection |
| `nats` | `NatsCheck`, the connection state and a flush on an async-nats client |

Everything comes from one crate, `sisa-health-checks`; no feature is on by default. Each part is
also its own crate (`sisa-health-checks-core`, `-axum`, `-postgres`, `-redis`, `-nats`).

## Quick start

```sh
cargo add sisa-health-checks --features axum,postgres,redis,nats
```

```rust,ignore
use sisa_health_checks::{
    EventLoopCheck, HealthRegistry, LIVE, NatsCheck, PostgresCheck, RedisCheck, Status,
    probe_routes, report_routes,
};

let registry = HealthRegistry::builder()
    // `live`: only the process itself. A failing liveness probe restarts the pod.
    .check("event-loop", EventLoopCheck::default())
    .tags([LIVE])
    // Checks are tagged `ready` by default: the pod gets no traffic while one fails.
    .check("postgres", PostgresCheck::new(pool))
    .check("nats", NatsCheck::new(nats))
    // An optional dependency: shown as degraded, never takes the pod out of rotation.
    .check("redis", RedisCheck::new(cache))
    .failure(Status::Degraded)
    .build()?;

// Runs every check now and every 5 seconds, until the handle drops.
let _poller = registry.spawn();

let admin = Router::new()
    .merge(probe_routes(registry.clone()))
    .merge(report_routes(registry.clone()));
```

On SIGTERM, call `registry.begin_draining()` and keep serving for a few seconds: `/readyz`
fails, load balancers stop sending traffic, and only then does the server stop.
[examples/axum-service](https://github.com/sisaio/sisa-health-checks-rs/tree/main/examples/axum-service)
does all of this.

## Your own checks

Implement `HealthCheck` with an `async fn`, or wrap a closure with `check_fn`:

```rust,ignore
use sisa_health_checks::{CheckContext, HealthCheck, HealthResult};

struct OutboxBacklog {
    pool: PgPool,
}

impl HealthCheck for OutboxBacklog {
    async fn check(&self, _context: &CheckContext) -> HealthResult {
        match sqlx::query_scalar::<_, i64>("SELECT count(*) FROM outbox WHERE sent_at IS NULL")
            .fetch_one(&self.pool)
            .await
        {
            Ok(backlog) if backlog > 10_000 => {
                HealthResult::degraded("outbox backlog").with_detail("backlog", backlog)
            }
            Ok(backlog) => HealthResult::healthy().with_detail("backlog", backlog),
            Err(error) => HealthResult::unhealthy("outbox unreadable").with_error(&error),
        }
    }
}
```

The registry runs it under its timeout (2 seconds by default), catches a panic, and runs it
again at the next poll; the check needs no timeout, retry or task of its own.

## What the endpoints answer

| Path | For | 200 | 503 | Body |
| --- | --- | --- | --- | --- |
| `/livez` | liveness and startup probes | every `live` check passes | one fails | `ok`, or `[+]`/`[-]` lines with `?verbose` |
| `/readyz` | readiness probe | every `ready` check passes, started, not draining | otherwise | same |
| `/health` | people and dashboards | no check unhealthy | one is | `application/health+json` |

```text
$ curl -i 'localhost:9090/readyz?verbose'
HTTP/1.1 503 Service Unavailable
content-type: text/plain; charset=utf-8

[+]postgres ok
[-]nats failed: connection is Disconnected
[+]redis degraded: PING failed
[+]shutdown ok
[+]poller ok
readyz check failed
```

The paths and text follow the Kubernetes API server's own `/livez` and `/readyz`. There is no
`/healthz`, which Kubernetes deprecated, and no `/startupz`: point the startup probe at `/livez`.
Serve `/health` on an admin port only, since it shows each check's details.

## Observability

Each run records `sisa.health.status` (1, 0.5 or 0) and `sisa.health.check.duration` per check
through the OpenTelemetry metrics API, and each change of status is one log line.
[docs/02-observability.md](https://github.com/sisaio/sisa-health-checks-rs/blob/main/docs/02-observability.md)
lists them and what to alert on.

## More

- [Docs](https://github.com/sisaio/sisa-health-checks-rs/blob/main/docs/00-overview.md): an
  overview, then architecture and observability, in reading order.
- [Roadmap](https://github.com/sisaio/sisa-health-checks-rs/blob/main/ROADMAP.md) and
  [changelog](https://github.com/sisaio/sisa-health-checks-rs/blob/main/CHANGELOG.md).
- [Contributing](https://github.com/sisaio/sisa-health-checks-rs/blob/main/CONTRIBUTING.md):
  building, testing and conventions.
- [Security](https://github.com/sisaio/sisa-health-checks-rs/blob/main/SECURITY.md): reporting
  a vulnerability.

Rust 1.89 or newer. MIT licensed.
