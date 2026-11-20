# ADR-0006: gRPC health runs on connectrpc and serves `Check` only

- Status: **Accepted**
- Date: 2026-11-20, when it was recorded here; settled before 0.0.1
- Deciders: Sang Au

## Context

Services that speak gRPC are probed through `grpc.health.v1.Health`. Clients reach it over gRPC,
gRPC-Web or Connect. The kubelet, Envoy and `grpc_health_probe` call `Check`; `Watch` streams
changes to clients that balance on health.

## Decision

- `sisa-health-checks-grpc` serves `grpc.health.v1.Health` on a connectrpc router.
- It serves `Check` only. `Watch` answers `UNIMPLEMENTED`, which the protocol allows.

## Consequences

- One router serves gRPC, gRPC-Web and Connect clients, so one crate covers all three.
- `Watch` needs a change notification in the core, and waits for a client that balances on
  health; it is on the [roadmap](../../ROADMAP.md), as is the service on tonic.

## Alternatives considered

- **tonic:** serves gRPC clients only; it may come beside the connectrpc one.
- **Serving `Watch` now:** no client that needs it has asked, and the core has no change
  notification yet.
