# Changelog

All notable changes are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- *(core)* `HealthRegistry`, which runs checks in the background and answers the `live` and
  `ready` probes and the health report from their latest results
- *(core)* the `HealthCheck` trait, `check_fn` for closures, and `EventLoopCheck`
- *(core)* per-check tags, timeouts and failure status, draining, `wait_for_start`, and stale
  results failing both probes
- *(core)* the `sisa.health.status` and `sisa.health.check.duration` metrics
- *(axum)* `/livez`, `/readyz` and `/health` routes
- *(postgres)* `PostgresCheck` on an sqlx pool
- *(redis)* `RedisCheck` on any redis-rs async connection
- *(nats)* `NatsCheck` on an async-nats client
