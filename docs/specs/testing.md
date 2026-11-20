# Crate: sisa-health-checks-testing

Status: **Implemented** · Published: no · Label: `area:testing` · Commit scope: `testing`

## Purpose

The test helpers the other crates' tests share: a server in Docker for one test, and a way to
wait until it answers. Never published, so it has no public API to keep.

## Public contract

| Item | Gives |
| --- | --- |
| `Server::postgres()` | A fresh PostgreSQL server for one test, user and password `postgres`, removed when the value drops |
| `Server::redis()` | The same for a Redis server |
| `Server::nats()` | The same for a NATS server |
| `eventually` | Calls an attempt until it succeeds, for a server that is still starting; panics with the last error after 120 attempts, half a second apart. An attempt has no deadline of its own, so one that never returns waits forever |

A server hands out its `port`, and can be stopped for a test that takes the server away.

## Behaviour

Every test starts a server of its own, so tests run side by side and share nothing. The image
comes from `POSTGRES_IMAGE`, `REDIS_IMAGE` and `NATS_IMAGE`, as `name:tag`, with the defaults
`postgres:17-alpine`, `redis:8-alpine` and `nats:2-alpine`.

## Guarantees and limits

The suite needs Docker and nothing else.

## Errors

None: a helper that can't start its server panics, which fails the test.

## Formats

None.
