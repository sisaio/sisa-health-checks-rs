# Overview

sisa-health-checks tells Kubernetes, load balancers and people whether a Rust service is working.
Register checks once at startup; a background poller runs them, and every endpoint answers from
the latest results without running anything itself.

## The words used everywhere

- **Check**: anything that implements `HealthCheck`, or a closure through `check_fn`, and returns
  a `HealthResult`: a status, a short description, details for the report, and the error behind a
  failure, which only the logs see.
- **Status**: `Healthy`, `Degraded` or `Unhealthy`, ordered from best to worst. Degraded passes
  the probes; unhealthy fails them.
- **Tag**: the probes a check counts for. `live` checks the process itself; `ready` checks the
  dependencies the pod can't serve without. A check is tagged `ready` unless given other tags,
  and a `report_only` check is in the report alone.
- **Failure status**: the worst status a check can report. An optional dependency's check gets
  `failure(Status::Degraded)`, so its outage shows without taking the pod out of rotation.
- **Registry**: `HealthRegistry`, built once with every check, and cheap to clone into handlers.
- **Poller**: the task `registry.spawn()` starts. It runs every check every `poll_interval_ms`
  (5 seconds by default), each under its timeout (`default_timeout_ms`, 2 seconds), and stops
  when its handle drops.
- **Probe**: the answer to `/livez` or `/readyz`: 200 or 503, and plain text in the Kubernetes
  API server's format.
- **Report**: every check's latest result at `/health`, in `application/health+json`.
- **Draining**: the state after `begin_draining()`, at shutdown, in which `/readyz` fails so
  traffic moves away before the server stops.

## Reading order

1. [Architecture](01-architecture.md): the crates, how a poll and a probe work, the rules the
   design depends on, the decisions already settled, and the exact formats.
2. [Observability](02-observability.md): the metrics and log lines, and what to alert on.

What comes next is in the [roadmap](../ROADMAP.md). The API docs on docs.rs come with examples.
To change the code, read [CONTRIBUTING.md](../CONTRIBUTING.md) after the architecture.
