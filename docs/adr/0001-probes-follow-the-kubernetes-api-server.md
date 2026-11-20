# ADR-0001: The probes follow the Kubernetes API server

- Status: **Accepted**
- Date: 2026-11-20, when it was recorded here; settled before 0.0.1
- Deciders: Sang Au

## Context

Kubernetes asks two questions of a pod: should it be restarted, and should it get traffic. The
API server answers them for itself on `/livez` and `/readyz`, in plain text that operators
already read with `kubectl get --raw /readyz?verbose`. Kubernetes deprecated `/healthz` in v1.16
for these two.

## Decision

- The probes are `/livez` and `/readyz`. There is no `/healthz` and no `/startupz`.
- A probe answers 200 to pass and 503 to fail.
- The body is plain text in the API server's format: `ok`, or the failing lines, and every line
  with `?verbose`. The exact text is in [Formats](../formats.md#probe-text).

## Consequences

- One name per meaning keeps probes unambiguous.
- The kubelet passes 200–399, and 503 is what load balancers and the API server use for not
  ready, so the answers need no translation.
- The startup probe asks the same question as liveness, only with more patience: point it at
  `/livez` with a generous `failureThreshold`.
- The probe text is a format tools read, so changing it is breaking.

## Alternatives considered

- **`/healthz`:** deprecated in Kubernetes, since one endpoint can't answer both questions.
- **`/startupz`:** the same answer as `/livez` under another name.
