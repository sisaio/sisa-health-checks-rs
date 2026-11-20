# Crate: sisa-health-checks-core

Status: **Implemented** in 0.0.1 · Published: yes · Label: `area:core` · Commit scope: `core`

## Purpose

The shared parts: the `HealthCheck` contract, the `HealthRegistry` that runs checks in the
background, and the probe answers and the report the protocol crates serve. It knows no web
framework and no client library, so a protocol crate or a check crate can be added without
touching it.

## Public contract

| Item | For |
| --- | --- |
| `HealthCheck`, `check_fn`, `CheckContext` | A check: a type with an `async fn check(&self, context)`, or a closure. The context gives the check's name and timeout, for a check that wants to pass a shorter deadline to its client |
| `HealthResult`, `Status` | What a check returns: a status, a short description, details for the report, and the error behind a failure, which only the logs see |
| `NamedCheck`, `ProvidesHealthChecks`, `LIVE`, `READY` | A check with its name, tags, timeout and failure status. A crate that owns several resources implements `ProvidesHealthChecks` and hands the application its checks, already named and tagged, to add with `extend` |
| `HealthRegistry`, `HealthRegistryBuilder`, `PollerHandle` | The registry, built once with every check and cheap to clone into handlers; `spawn()` starts its poller, which stops when the handle drops |
| `HealthSettings` | `poll_interval_ms` (5 seconds by default) and `default_timeout_ms` (2 seconds) |
| `Probe`, `ProbeReport`, `ProbeLine` | The answer to `/livez` or `/readyz`, from the stored results |
| `HealthReport`, `CheckReport` | Every check's latest result, for `/health` |
| `EventLoopCheck` | A check of the process itself: whether the Tokio runtime wakes tasks on time |
| `metrics` | The metric names and attributes, as [Observability](../observability.md#metrics) lists them |

Why a check is a trait with `async fn`: [ADR-0005](../adr/0005-a-check-is-a-trait-with-async-fn.md).

## Behaviour

The poller runs every check every `poll_interval_ms`, each under its timeout, and stores the
results; [Flows](../flows.md#a-poll-and-a-probe) draws a poll. Each result is capped at the
check's failure status, changes are logged and the metrics recorded, as
[Observability](../observability.md) lists.

A probe reads the stored results under a read lock and builds its answer. On top of the checks
with its tag it adds:

| Line | On | Fails when |
| --- | --- | --- |
| `startup` | `/readyz`, with `wait_for_start()` | `mark_started()` hasn't been called |
| `shutdown` | `/readyz` | `begin_draining()` was called |
| `poller` | both | the last poll ended more than 3 × `poll_interval_ms` + the longest timeout ago |

A check that hasn't run yet is `pending`: it passes `/livez` and fails `/readyz`, so a starting
pod is neither restarted nor sent traffic.

Why results come from a poller, and why degraded passes:
[ADR-0004](../adr/0004-results-from-a-background-poller.md).

## Guarantees and limits

The library's guarantees are in [Architecture](../architecture.md#guarantees-and-limits), with
the [rules](../architecture.md#rules-that-hold-the-design-together) the registry keeps: it
applies each check's timeout and catches its panic, and only `run()` calls a check.

## Errors

| Type | Says |
| --- | --- |
| `HealthError` | Why a registry couldn't be built: a setting out of range, a check name that isn't allowed or is used twice, a check's options out of range, or an option given before any check. Running checks never fails: a failing check is a result, not an error |

## Formats

The [probe text](../formats.md#probe-text) and the [report](../formats.md#report), and the
[metrics](../observability.md#metrics) and [logs](../observability.md#logs).
