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
  results failing both probes and the report
- *(core)* the `sisa.health.status` and `sisa.health.check.duration` metrics
- *(core)* `http::probe` and `http::report`: the probes and the report as plain HTTP answers, for
  any web framework
- *(axum)* `/livez`, `/readyz` and `/health` routes
- *(grpc)* the `grpc.health.v1.Health` service on a connectrpc router, for gRPC and Connect
  clients: `Check` for the `livez` and `readyz` services
- *(postgres)* `PostgresCheck` on an sqlx pool
- *(redis)* `RedisCheck` on any redis-rs async connection
- *(nats)* `NatsCheck` on an async-nats client
