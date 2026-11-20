# ADR-0004: Results come from a background poller, and degraded passes

- Status: **Accepted**
- Date: 2026-11-20, when it was recorded here; settled before 0.0.1
- Deciders: Sang Au

## Context

A probe that runs its checks ties dependency load to probe traffic: a burst of probes, or several
probers, multiplies the calls to every dependency, and a slow dependency holds probes open. A
check can also find a dependency working but impaired.

## Decision

- A background poller runs the checks on an interval and stores the results. Probes and the
  report read the stored results and never run a check.
- A degraded check passes the probes. Only an unhealthy one fails them.

## Consequences

- Probe traffic and dependency load are independent, and a probe answers in microseconds.
- Results are as old as one poll interval plus the time the poll took: a poll stores nothing
  until its slowest check ends, which can take that check's whole timeout. A dead poller must not
  leave the last good answer in place: stale results fail both probes and the report.
- A degraded dependency is worth seeing, not worth losing traffic over: it shows in the report,
  the logs and the metrics.

## Alternatives considered

- **Running the checks per request:** probe traffic becomes dependency load, and a slow
  dependency piles up probes.
- **Degraded fails readiness:** an impaired optional dependency would take the pod out of
  rotation.
