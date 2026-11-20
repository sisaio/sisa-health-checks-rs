# Flows

Diagrams of each step that the [core spec](specs/core.md#behaviour) describes, across the crates.
A new flow follows [templates/flow-spec.md](templates/flow-spec.md).

## A poll and a probe

```mermaid
sequenceDiagram
    participant Poller
    participant Registry
    participant Check
    participant Kubelet
    loop every poll_interval_ms
        Poller->>Registry: run()
        par each check
            Registry->>Check: check(context), under its timeout
            Check-->>Registry: HealthResult
        end
        Registry->>Registry: cap at failure status, store, log changes, record metrics
    end
    Kubelet->>Registry: GET /readyz
    Registry-->>Kubelet: 200 or 503 from the stored results
```

`run()` starts every check at once and waits for all of them, so a poll takes as long as its slowest
check, and no longer than the longest timeout as long as every check awaits: a timeout can't
interrupt a check that blocks its thread ([Guarantees and
limits](architecture.md#guarantees-and-limits)). A check that times out or panics becomes an
unhealthy result with the message `timed out after <n> ms` or `the check panicked`; the others are
unaffected.

A probe reads the stored results under a read lock and builds its answer, with the lines the
[core spec](specs/core.md#behaviour) lists on top of the checks with its tag.
