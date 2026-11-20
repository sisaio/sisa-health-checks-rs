# Crate: sisa-health-checks-http

Status: **Implemented** in 0.0.1 · Published: yes · Label: `area:http` · Commit scope: `http`

## Purpose

The HTTP answers in plain data, for any web framework: `/livez` and `/readyz` for Kubernetes
probes, and `/health` for people and dashboards. It knows no web framework and does no routing:
an application copies each answer into its own response
([ADR-0002](../adr/0002-http-answers-as-plain-data.md)).

## Public contract

| Item | For |
| --- | --- |
| `probe(registry, probe, query)` | The `/livez` or `/readyz` answer; the query turns on `?verbose` |
| `report(registry)` | The `/health` answer |
| `HttpAnswer` | A status code, a content type, a `Cache-Control` value and a body |
| `LIVE_PATH`, `READY_PATH`, `REPORT_PATH` | `/livez`, `/readyz` and `/health`, for the framework's router |

## Behaviour

| Request | Call | Answers | Body |
| --- | --- | --- | --- |
| `GET /livez` | `probe` with `Probe::Live` | 200 unless a check tagged `live` fails or the results are stale | `ok`, or the failing lines |
| `GET /readyz` | `probe` with `Probe::Ready` | 200 unless a check tagged `ready` fails, the results are stale, the registry drains, or it isn't started | `ok`, or the failing lines |
| `GET /livez?verbose`, `GET /readyz?verbose` | `probe` with the query | The same status | Every line, as `[+]postgres ok` |
| `GET /health` | `report` | 503 when any check is unhealthy or the results are stale, else 200 | `application/health+json` with every check |

Every answer has `Cache-Control: no-store`, so no proxy serves a stale one.

## Guarantees and limits

The library's guarantees are in [Architecture](../architecture.md#guarantees-and-limits). The
probe answers show each check's name, whether it passed and, when it failed or is degraded, its
description, which a check writes to be safe to show, so they can stay open to the kubelet. The
report shows details, so serve it behind your own authorization, or on an admin port only.

## Errors

None: an answer is always built, and a failing probe is a 503, not an error.

## Formats

The [probe text](../formats.md#probe-text) and the [report](../formats.md#report). Why these
paths and this text: [ADR-0001](../adr/0001-probes-follow-the-kubernetes-api-server.md) and
[ADR-0003](../adr/0003-the-report-format.md).
