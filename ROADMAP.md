# Roadmap

What shipped is in the [changelog](CHANGELOG.md). The issues and their milestones show the work in
progress. Each milestone is a release, and the milestones on GitHub come from
[.github/milestones.yml](.github/milestones.yml).

| Milestone | Status | Goal |
| --- | --- | --- |
| 0.1.0 | Next | The registry, the probes and the report, their HTTP answers for any web framework, the gRPC health service, and checks for PostgreSQL, Redis and NATS. 0.0.1, on crates.io, has all of these |
| 0.2.0 | Planned | Excluding checks per request, as the Kubernetes API server does with `?exclude=<name>`, and one path per check, such as `/readyz/postgres`, for debugging a single dependency |

## Later

Ideas that wait until users ask for them:

- More checks, each its own crate: Kafka, RabbitMQ, an HTTP dependency, disk space.
- A check that turns unhealthy after it has been degraded for a set time.
- `Watch` on the gRPC health service, for clients that balance on health.
- The gRPC health service on tonic, beside the connectrpc one.
