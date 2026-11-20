# Formats

What the library answers with: the probe text, the report and the gRPC health answers. Tools and
people read these, so they are part of the public API, and a change to one is breaking. The
metric names and attributes fall under the same rule and are in
[Observability](observability.md#metrics).

## Probe text

`Content-Type: text/plain; charset=utf-8`, `Cache-Control: no-store`. A passing probe without
`?verbose` answers `ok`. Otherwise every line of the probe with `?verbose`, or only the failing
lines without it, then the verdict:

```text
[+]<name> ok
[+]<name> degraded: <description>
[-]<name> failed: <description>
[-]<name> pending
readyz check failed
```

The last line is `<livez|readyz> check passed` or `<livez|readyz> check failed`.

## Report

`GET /health`, `Content-Type: application/health+json`, 503 when a check is unhealthy or the
results are stale, and 200 otherwise:

```json
{
  "status": "warn",
  "checks": {
    "postgres": [
      {
        "status": "pass",
        "observedValue": 1.8,
        "observedUnit": "ms",
        "tags": ["ready"],
        "details": { "pool_size": 4, "pool_idle": 3 }
      }
    ],
    "redis": [
      {
        "status": "warn",
        "observedValue": 2000.0,
        "observedUnit": "ms",
        "output": "timed out after 2000 ms",
        "tags": ["ready"]
      }
    ]
  }
}
```

`status` is `pass`, `warn` or `fail` for healthy, degraded and unhealthy. A check that hasn't run
is `fail` with `output` `not checked yet`. While draining, the report adds
`"notes": ["draining: /readyz fails"]`. Once the results are stale, the top `status` is `fail`
whatever the checks last found, each check keeps its last result, and `notes` gains
`"stale: no new results; /livez and /readyz fail"`.

## gRPC health

`grpc.health.v1.Health/Check`, for gRPC, gRPC-Web and Connect clients. The request's `service`
picks the probe, and the answer is that probe's verdict, with no check name or description:

| `service` | Answers from | `SERVING` | `NOT_SERVING` |
| --- | --- | --- | --- |
| `""` or `readyz` | `/readyz` | it would answer 200 | it would answer 503 |
| `livez` | `/livez` | it would answer 200 | it would answer 503 |
| any other name | | | the call fails with `NOT_FOUND` |

`Watch` fails with `UNIMPLEMENTED`.
