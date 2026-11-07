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

## Logs

| Level | Message | Fields | When |
| --- | --- | --- | --- |
| `info` | `health check improved` | `check`, `status`, `description` | A check's status got better than its previous run |
| `warn` | `health check got worse` | `check`, `status`, `description`, `error` | A check's status got worse, including its first run if not healthy |

A check that stays down logs once, when it went down, not at every poll. Each run is also a
`debug` span, `health check`, with the `check` field, so a check's own client spans nest under it.

## What to alert on

| Alert | Query, in PromQL terms | Why |
| --- | --- | --- |
| A dependency is down | `min by (check) (sisa_health_status) == 0` for 2 minutes | Every pod sees it, so it is the dependency, not a pod |
| One pod is unhealthy | `sisa_health_status == 0` on some pods only, for 5 minutes | Points at that pod's connection or network |
| A check is slow | p99 of `sisa_health_check_duration` near its timeout | Timeouts are about to start |
| No points at all | `absent(sisa_health_status)` | The poller or the exporter stopped |

Leave restarts and traffic to the probes; alert on the metrics, which keep history across pods.
