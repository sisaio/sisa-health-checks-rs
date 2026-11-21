# Observability

## Metrics

Recorded through the OpenTelemetry metrics API under the meter `sisa-health-checks`. Nothing is
recorded until the application installs an SDK meter provider; install it before the first poll.

| Metric | Type | Attributes | What it records |
| --- | --- | --- | --- |
| `sisa.health.status` | gauge | `check` | Each check's latest status: 1 healthy, 0.5 degraded, 0 unhealthy |
| `sisa.health.check.duration` | histogram, seconds | `check`, `status` | How long each run took; a timed-out run records the full timeout |

Each poll records one point per check on each, so the series per pod are a fixed set: one per
check name, and three statuses on the histogram.

### Getting them out

The library crates use the OpenTelemetry API only; the application picks the SDK and the exporter.
Any SDK works when it is built on the same `opentelemetry` version as this library, 0.33: a
provider from another version is a different global, and the registry records into nothing.

[examples/axum-service](../examples/axum-service/src/main.rs) uses
[sisa-telemetry](https://crates.io/crates/sisa-telemetry):

```rust,ignore
// First thing in `main`, inside the Tokio runtime and before the registry is built.
let telemetry = sisa_telemetry::init(
    &TelemetrySettings::default(),
    ServiceResource::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
)?;

// ... build the registry, spawn the poller, serve ...

// After the server has stopped. A Collector that is down makes this an error: log it.
if let Err(error) = telemetry.shutdown(Duration::from_secs(2)) {
    tracing::warn!(%error, "the last telemetry was not sent");
}
```

| What | Default | Change it with |
| --- | --- | --- |
| Where the metrics go | OTLP over gRPC to a Collector at `http://localhost:4317`, every 30 seconds | `otlp.endpoint`, `otlp.protocol` and `metrics.interval` in `TelemetrySettings`, or the standard `OTEL_*` variables, which win |
| The logs | JSON on stdout at `info` | `log.format` and `log.filter` |

While no Collector answers, the service runs the same and each failed export is logged.
sisa-telemetry 0.1.0 has limits: the logs go to stdout only, not over OTLP; the export has no TLS,
so the Collector is a sidecar or on the node; and `RUST_LOG` is not read, the filter is
`log.filter`.

In a test, `sisa_telemetry::testing::capture()`, from its `testing` feature, takes the place of
`init` and keeps the metrics in memory. Call it before the registry's first poll, in a test binary
of its own, as the example's
[tests/metrics.rs](../examples/axum-service/tests/metrics.rs) does.

## Logs

| Level | Message | Fields | When |
| --- | --- | --- | --- |
| `info` | `health check improved` | `check`, `status`, `description` | A check's status got better than its previous run |
| `warn` | `health check got worse` | `check`, `status`, `description`, `error` | A check's status got worse, including its first run if not healthy |

A check that stays down logs once, when it went down, not at every poll. Each run is also a
`debug` span, `health check`, with the `check` field, so a check's own client spans nest under it.

The lines go through [`tracing`](https://docs.rs/tracing), so they need a subscriber, which the
application installs; `sisa_telemetry::init` installs one. Its default filter is `info`, which
shows both lines and leaves the `debug` span out.

## What to alert on

| Alert | Query, in PromQL terms | Why |
| --- | --- | --- |
| A dependency is down | `min by (check) (sisa_health_status) == 0` for 2 minutes | Every pod sees it, so it is the dependency, not a pod |
| One pod is unhealthy | `sisa_health_status == 0` on some pods only, for 5 minutes | Points at that pod's connection or network |
| A check is slow | p99 of `sisa_health_check_duration_seconds` near its timeout | Timeouts are about to start |
| No points at all | `absent(sisa_health_status)` | The exporter stopped, or no pod is running |

A poller that stops doesn't show in the metrics: the gauge keeps its last value, and the SDK
goes on exporting it. The probes and the report catch it instead: they fail once the results are
stale.

Leave restarts and traffic to the probes; alert on the metrics, which keep history across pods.
