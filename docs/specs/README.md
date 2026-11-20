# Specs

What each crate does, written so it can be built, reviewed and tested against. One page per
crate, all with the same sections: purpose, public contract, behaviour, guarantees and limits,
errors, and formats.

| Crate | Spec | Holds |
| --- | --- | --- |
| `sisa-health-checks` | [facade.md](facade.md) | The re-exports and the features that pick the parts |
| `sisa-health-checks-core` | [core.md](core.md) | The check contract, the registry and its poller, the probes, the report, the metrics |
| `sisa-health-checks-http` | [http.md](http.md) | The `/livez`, `/readyz` and `/health` answers as plain data |
| `sisa-health-checks-grpc` | [grpc.md](grpc.md) | The `grpc.health.v1.Health` service, on connectrpc |
| `sisa-health-checks-postgres` | [postgres.md](postgres.md) | The PostgreSQL check |
| `sisa-health-checks-redis` | [redis.md](redis.md) | The Redis check |
| `sisa-health-checks-nats` | [nats.md](nats.md) | The NATS check |
| `sisa-health-checks-testing` | [testing.md](testing.md) | The test servers and helpers; not published |

What spans the crates is one level up: how they fit in [Architecture](../architecture.md), the
probe text, the report and the gRPC answers in [Formats](../formats.md), and the steps in
[Flows](../flows.md).

Changing what a crate does: update its page in the same PR. A new crate starts from
[templates/crate-spec.md](../templates/crate-spec.md). A choice that is hard to reverse gets an
[ADR](../adr/README.md). Planned work goes in the [roadmap](../../ROADMAP.md), not here: a spec
says what the code does, and marks what is still open as an open question.
