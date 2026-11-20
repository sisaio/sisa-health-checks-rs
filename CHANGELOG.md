# Changelog

All notable changes are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.0.1](https://github.com/sisaio/sisa-health-checks-rs/releases/tag/v0.0.1) - 2026-11-20

### Added

- *(http)* [**breaking**] move the HTTP answers to their own crate, drop axum
- *(core)* give the probes and the report as plain HTTP answers
- *(grpc)* serve grpc.health.v1.Health from the registry
- add health registry, probes, and postgres, redis and nats checks

### Changed

- *(core)* log and record metrics outside the results lock

### Fixed

- *(core)* [**breaking**] fail the report when its results are stale
- *(core)* catch a panic raised before a check's future

### Other

- Initial commit

### Added

- *(core)* `HealthRegistry`, which runs checks in the background and answers the `live` and
  `ready` probes and the health report from their latest results
- *(core)* the `HealthCheck` trait, `check_fn` for closures, and `EventLoopCheck`
- *(core)* per-check tags, timeouts and failure status, draining, `wait_for_start`, and stale
  results failing both probes and the report
- *(core)* the `sisa.health.status` and `sisa.health.check.duration` metrics
- *(http)* `probe` and `report`: the `/livez`, `/readyz` and `/health` answers as plain data, for
  any web framework
- *(grpc)* the `grpc.health.v1.Health` service on a connectrpc router, for gRPC and Connect
  clients: `Check` for the `livez` and `readyz` services
- *(postgres)* `PostgresCheck` on an sqlx pool
- *(redis)* `RedisCheck` on any redis-rs async connection
- *(nats)* `NatsCheck` on an async-nats client
