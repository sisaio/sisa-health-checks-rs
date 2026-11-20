# ADR-0005: A check is a trait with `async fn`, erased inside the registry

- Status: **Accepted**
- Date: 2026-11-20, when it was recorded here; settled before 0.0.1
- Deciders: Sang Au

## Context

Users write their own checks, and the registry holds checks of many types in one list.

## Decision

`HealthCheck` is a trait with an `async fn check`. The registry erases the type internally.

## Consequences

- Users write plain async code, with no boxing and no macro.
- The boxing stays inside the registry, out of the public API.

## Alternatives considered

- **A trait that returns a boxed future:** puts the boxing in every user's check.
