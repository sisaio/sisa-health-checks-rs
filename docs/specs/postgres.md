# Crate: sisa-health-checks-postgres

Status: **Implemented** in 0.0.1 · Published: yes · Label: `area:postgres` · Commit scope: `postgres`

## Purpose

A PostgreSQL check on the application's own sqlx pool. It opens no connection of its own.

## Public contract

`PostgresCheck::new(pool)`, which implements core's [`HealthCheck`](core.md#public-contract), and
the `sqlx` crate it is built on, re-exported.

## Behaviour

The check runs `SELECT 1` on a connection from the pool, so it fails when the pool can't reach
the server or has no connection to spare. A failure is unhealthy with the description
`SELECT 1 failed`; the error goes to the logs. The result carries the details `pool_size` and
`pool_idle`, so a pool that runs out shows in the report before requests time out.

## Guarantees and limits

The library's guarantees are in [Architecture](../architecture.md#guarantees-and-limits). The
check waits for a connection like any query does, so keep the pool's acquire timeout under the
check's timeout to see the pool's own error.

## Errors

None of its own: a failure is an unhealthy `HealthResult`.

## Formats

Its details show in the [report](../formats.md#report).
