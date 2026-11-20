# ADR-0002: HTTP answers are plain data, with no crate per web framework

- Status: **Accepted**
- Date: 2026-11-20, when it was recorded here; settled before 0.0.1
- Deciders: Sang Au

## Context

Web frameworks don't share one response type: axum, warp and poem are on `http` 1, actix-web 4
and Rocket 0.5 on `http` 0.2, as checked for 0.0.1. The library started with an axum crate of
routes, dropped before 0.0.1, which tied it to one major version of one framework.

## Decision

- `sisa-health-checks-http` gives each answer as plain data: a status code, a content type, a
  `Cache-Control` value and a body. It uses no `http` crate types.
- There is no crate per web framework. An application writes the handlers that copy an answer
  into its own response; the http crate's docs and the example service show them.

## Consequences

- A number and three strings fit every framework, with no dependency.
- A framework's new major version needs no release of this library.
- The handlers are a few lines an application writes once, and routing stays with the framework.

## Alternatives considered

- **`http` crate types in the answer:** serves the frameworks on one `http` major version only.
- **A crate of routes per framework:** needs a breaking release for each major version of that
  framework, and serves one version at a time.
