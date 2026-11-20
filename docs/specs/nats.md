# Crate: sisa-health-checks-nats

Status: **Implemented** in 0.0.1 · Published: yes · Label: `area:nats` · Commit scope: `nats`

## Purpose

A NATS check on the application's own async-nats client. It opens no connection of its own.

## Public contract

`NatsCheck::new(client)`, which implements core's [`HealthCheck`](core.md#public-contract), and
the `async_nats` crate, re-exported.

## Behaviour

The connection must be up, and a flush of the client's pending writes must succeed. A client
that isn't connected is unhealthy with the description `connection is <state>`; a flush that
fails is unhealthy with `flush failed`. The error is logged when the check's status gets worse,
not on every failed run.

## Guarantees and limits

The library's guarantees are in [Architecture](../architecture.md#guarantees-and-limits). The
flush shows that the client's connection task is alive and its socket still takes writes, not
that the server answered: async-nats has no ping an application can send. A connection that died
without closing is found by the client's own pings, and fails this check once the client reports
it as disconnected.

Tag it `ready` when the pod consumes from NATS. When it only publishes through an outbox, the
outbox holds messages while NATS is down, so give it `failure(Status::Degraded)` instead.

## Errors

None of its own: a failure is an unhealthy `HealthResult`.

## Formats

None of its own: its result shows in the [report](../formats.md#report).
