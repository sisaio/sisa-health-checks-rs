# Crate: sisa-health-checks-redis

Status: **Implemented** in 0.0.1 · Published: yes · Label: `area:redis` · Commit scope: `redis`

## Purpose

A Redis check through the application's own redis-rs connection, which works on Redis, Valkey,
Dragonfly and Garnet, and on a cluster connection too. It opens no connection of its own.

## Public contract

`RedisCheck::new(connection)`, on any redis-rs async connection, which implements core's
[`HealthCheck`](core.md#public-contract), and the `redis` crate, re-exported.

## Behaviour

The check sends `PING`. A failure is unhealthy with the description `PING failed`, and the error
goes to the logs; any answer other than the expected one is unhealthy with
`PING answered <answer>`.

## Guarantees and limits

The library's guarantees are in [Architecture](../architecture.md#guarantees-and-limits). `PING`
proves the server answers, not that it accepts writes. For that, write your own check that sets a
key of its own with a short TTL and reads it back.

## Errors

None of its own: a failure is an unhealthy `HealthResult`.

## Formats

None of its own: its result shows in the [report](../formats.md#report).
