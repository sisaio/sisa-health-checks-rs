# sisa-health-checks docs

sisa-health-checks tells Kubernetes, load balancers and people whether a Rust service is working.
Register checks once at startup; a background poller runs them, and every endpoint answers from
the latest results without running anything itself. Add the `sisa-health-checks` crate with the
features of the parts you use; none is on by default.

## Layout

```text
docs/
├── README.md          this page: the index and the words used everywhere
├── architecture.md    how the crates fit: crate map, guarantees, rules
├── formats.md         the probe text, the report and the gRPC answers
├── flows.md           diagrams of a poll and a probe
├── observability.md   the metrics and log lines, and what to alert on
├── specs/             what each crate does: one page per crate
├── adr/               architecture decision records: why it is this way
└── templates/         the crate spec and the flow; the ADR template is in adr/
```

At the repository root: [ROADMAP.md](../ROADMAP.md) for what comes next,
[CHANGELOG.md](../CHANGELOG.md) for what shipped, [CONTRIBUTING.md](../CONTRIBUTING.md) for the
checks, code style and Git rules, [SECURITY.md](../SECURITY.md), and [AGENTS.md](../AGENTS.md) for
coding agents. What each type does for users is in its Rust docs, not here.

## Where to start

| You want to | Read |
| --- | --- |
| See how the parts fit and what the library promises | [architecture.md](architecture.md) |
| Change or use one crate | Its page in [specs/](specs/README.md) |
| Read a probe's text, the report or the gRPC answer from another tool | [formats.md](formats.md) |
| Follow a poll or a probe step by step | [flows.md](flows.md) |
| Read the metrics and logs, or set up alerts | [observability.md](observability.md) |
| Know why something was chosen, before proposing another way | [adr/](adr/README.md) |
| Know what's next | [ROADMAP.md](../ROADMAP.md) |
| Contribute, commit, release | [CONTRIBUTING.md](../CONTRIBUTING.md) |

## Words used everywhere

| Word | Meaning |
| --- | --- |
| **Check** | Anything that implements `HealthCheck`, or a closure through `check_fn`, and returns a `HealthResult`: a status, a short description, details for the report, and the error behind a failure, which only the logs see |
| **Status** | `Healthy`, `Degraded` or `Unhealthy`, ordered from best to worst. Degraded passes the probes; unhealthy fails them |
| **Tag** | The probes a check counts for. `live` checks the process itself; `ready` checks the dependencies the pod can't serve without. A check is tagged `ready` unless given other tags, and a `report_only` check is in the report alone |
| **Failure status** | The worst status a check can report. An optional dependency's check gets `failure(Status::Degraded)`, so its outage shows without taking the pod out of rotation |
| **Registry** | `HealthRegistry`, built once with every check, and cheap to clone into handlers |
| **Poller** | The task `registry.spawn()` starts. It runs every check every `poll_interval_ms` (5 seconds by default), each under its timeout (`default_timeout_ms`, 2 seconds), and stops when its handle drops |
| **Probe** | The answer to `/livez` or `/readyz`: 200 or 503, and plain text in the Kubernetes API server's format |
| **Report** | Every check's latest result at `/health`, in `application/health+json` |
| **Draining** | The state after `begin_draining()`, at shutdown, in which `/readyz` fails so traffic moves away before the server stops |

## How docs grow

Docs change in the same PR as the code they describe.

1. **A change to what a crate does:** update its page in [specs/](specs/README.md), and the
   diagram in [flows.md](flows.md) if a step changes.
2. **A change to a route, the probe text, the report's JSON or a gRPC service name:** update
   [formats.md](formats.md). It is part of the public API, so the change is breaking. A metric or
   a log line changes in [observability.md](observability.md), under the same rule.
3. **A new crate:** copy [templates/crate-spec.md](templates/crate-spec.md) to
   `specs/<crate>.md`, and add it to the [specs index](specs/README.md) and the
   [crate map](architecture.md#crates).
4. **A new flow across crates:** add a section to [flows.md](flows.md), in the shape of
   [templates/flow-spec.md](templates/flow-spec.md).
5. **A choice that is hard to reverse, or that changes a rule or a dependency:** write an
   [ADR](adr/README.md). An accepted ADR is never rewritten; a new one supersedes it.

A spec's `Status` moves from Draft to Accepted to Implemented.
