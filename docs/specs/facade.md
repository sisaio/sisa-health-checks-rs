# Crate: sisa-health-checks

Status: **Implemented** in 0.0.1 · Published: yes · Label: `area:facade` · Commit scope: `facade`

## Purpose

The crate applications depend on. It re-exports what an application uses from the other crates
and has no code of its own, so each part is also usable as its own crate.

## Public contract

Everything an application uses from [core](core.md) and, behind a feature each, the protocols and
the checks. No feature is on by default: an application names the protocols and the checks it
uses.

| Feature | Adds |
| --- | --- |
| `http` | The `http` module: the [`/livez`, `/readyz` and `/health` answers](http.md) as plain data, for any web framework |
| `grpc` | `grpc_routes`: [`grpc.health.v1.Health`](grpc.md) for gRPC and Connect clients, on connectrpc |
| `postgres` | `PostgresCheck`, the [PostgreSQL check](postgres.md) on an sqlx pool, and the `sqlx` crate it is built on |
| `redis` | `RedisCheck`, the [Redis check](redis.md) on any redis-rs async connection, and the `redis` crate |
| `nats` | `NatsCheck`, the [NATS check](nats.md) on an async-nats client, and the `async_nats` crate |

## Behaviour

None of its own. CI builds the crate alone with each feature, so one feature never needs
another.

## Guarantees and limits

The library's guarantees are in [Architecture](../architecture.md#guarantees-and-limits). The
sqlx, redis-rs, async-nats and connectrpc types that reach the API through the re-exports are
part of the public API: moving to a release of theirs that breaks their own API is breaking here
too.

## Errors

None of its own: it re-exports the [core](core.md#errors) error type.

## Formats

None of its own: see [Formats](../formats.md).
