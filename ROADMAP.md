# Roadmap

What shipped is in the [changelog](CHANGELOG.md). The issues and their milestones show the work in
progress.

## 0.1.0

The registry, the probes and the report, the axum routes, and checks for PostgreSQL, Redis and
NATS.

## 0.2.0

- Excluding checks per request, as the Kubernetes API server does with `?exclude=<name>`.
- One path per check, such as `/readyz/postgres`, for debugging a single dependency.
- Ready-made routes for other frameworks where users ask, as the axum crate is: actix-web, or a
  plain `tower` service. Any framework can already serve the `http` module's answers.

## Later

Ideas that wait until users ask for them:

- More checks, each its own crate: Kafka, RabbitMQ, an HTTP dependency, disk space.
- A check that turns unhealthy after it has been degraded for a set time.
- A gRPC health service (`grpc.health.v1.Health`) on the same registry.
