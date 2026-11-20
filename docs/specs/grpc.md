# Crate: sisa-health-checks-grpc

Status: **Implemented** in 0.0.1 · Published: yes · Label: `area:grpc` · Commit scope: `grpc`

## Purpose

The standard gRPC health service, `grpc.health.v1.Health`, on a connectrpc router. One router
serves gRPC, gRPC-Web and Connect clients, so a kubelet `grpc:` probe, Envoy, `grpc_health_probe`
and a Connect client all get the same answer
([ADR-0006](../adr/0006-grpc-health-on-connectrpc.md)).

## Public contract

| Item | For |
| --- | --- |
| `grpc_routes(registry)` | The router with the health service, to merge into the application's own |
| `LIVE_SERVICE`, `READY_SERVICE` | `livez` and `readyz`, the service names a probe asks for |

## Behaviour

`Check` answers from the registry's latest results, as `/livez` and `/readyz` do, and never runs
a check. The request's service name picks the probe:

| `service` | Answers | `SERVING` when |
| --- | --- | --- |
| `""` or `readyz` | readiness | every check tagged `ready` passes, started, not draining |
| `livez` | liveness | every check tagged `live` passes |
| any other name | | never: the call fails with `NOT_FOUND` |

Otherwise the answer is `NOT_SERVING`. Both are `NOT_SERVING` once the results are stale.

`Watch` isn't served: it answers `UNIMPLEMENTED`, which the protocol allows.

## Guarantees and limits

The library's guarantees are in [Architecture](../architecture.md#guarantees-and-limits). The
answer carries no check name or description, so the service can stay open to the kubelet.

## Errors

No error type of its own: an unknown service name fails the call with `NOT_FOUND`, and `Watch`
with `UNIMPLEMENTED`.

## Formats

[gRPC health](../formats.md#grpc-health).
