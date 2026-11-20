# Architecture decision records

Short records of significant decisions: why, not only what. The rules:

1. Copy [0000-template.md](0000-template.md) to the next number, such as `0007-short-title.md`.
2. Open it as **Proposed** in the PR that needs it. It becomes **Accepted** when the PR merges.
3. Never rewrite an accepted ADR. Write a new one and mark the old one **Superseded by
   ADR-XXXX**.
4. Write an ADR when a choice is hard to reverse, changes a rule, a data format or a dependency,
   or someone will ask "why did we do this?" in six months.

Don't re-propose an accepted decision without new evidence.

| # | Decision | Status |
| --- | --- | --- |
| [0001](0001-probes-follow-the-kubernetes-api-server.md) | `/livez` and `/readyz` with no `/healthz` or `/startupz`; 200 to pass and 503 to fail; plain text in the API server's format | Accepted |
| [0002](0002-http-answers-as-plain-data.md) | HTTP answers are plain data, not `http` crate types, and there is no crate per web framework | Accepted |
| [0003](0003-the-report-format.md) | The report is `application/health+json`, served apart from the probes | Accepted |
| [0004](0004-results-from-a-background-poller.md) | Results come from a background poller, not per request; degraded passes the probes | Accepted |
| [0005](0005-a-check-is-a-trait-with-async-fn.md) | A check is a trait with `async fn`, erased inside the registry | Accepted |
| [0006](0006-grpc-health-on-connectrpc.md) | gRPC health runs on connectrpc and serves `Check` only, no `Watch` | Accepted |
