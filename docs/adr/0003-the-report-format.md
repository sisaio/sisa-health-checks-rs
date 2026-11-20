# ADR-0003: The report is `application/health+json`, apart from the probes

- Status: **Accepted**
- Date: 2026-11-20, when it was recorded here; settled before 0.0.1
- Deciders: Sang Au

## Context

People and dashboards want every check's latest result, with details the probes leave out. The
closest thing to a standard for a health document is `application/health+json`, an expired IETF
draft (draft-inadarei-api-health-check), not an RFC.

## Decision

- The report is served in `application/health+json`, and the draft is followed as a draft. Its
  shape is in [Formats](../formats.md#report).
- The report is an answer of its own, apart from the probes. The application routes each answer,
  so it chooses where the report is served.

## Consequences

- Tools that read the draft's format read the report.
- The report shows details meant for an admin port, so an application can serve it behind its own
  authorization while the probes stay open to the kubelet.
- The draft may never become an RFC; the report's JSON is this library's format to keep stable.

## Alternatives considered

- **A format of our own:** nothing existing would read it.
- **The details in the probe answers:** the probes are open to the kubelet, and details belong
  behind authorization.
