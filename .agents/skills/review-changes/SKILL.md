---
name: review-changes
description: Reviews a diff or pull request in sisa-health-checks-rs, code and docs alike, against CI's checks, the design rules, the settled decisions and the docs conventions, and reports findings by severity. Use when asked to review, evaluate or check a change, a branch, a PR or a docs page, even if the word "review" isn't used.
---

# Review changes

Review what changed, not the whole crate, unless asked to review the whole repository (below).
Report findings; don't push fixes unless asked.

## Workflow

- [ ] Get the diff: the PR's diff, or `git diff main...HEAD` on a branch.
- [ ] Run the checks in [CONTRIBUTING.md](../../../CONTRIBUTING.md#build-and-check). Note each
      failure with its first error.
- [ ] Check the diff against the
      [rules](../../../docs/architecture.md#rules-that-hold-the-design-together), the
      [decisions](../../../docs/adr/README.md) and the
      [hard rules](../../../AGENTS.md#hard-rules) in AGENTS.md. The ones most often broken:
  - a probe or the report never runs a check: they read the poller's latest results;
  - a check shipped for `live` never touches a dependency;
  - a response never carries a result's error, only its description;
  - no new dependency in a library crate.
- [ ] Check what goes with the change: a test for each new behavior, doc comments on new public
      items, the crate's [spec](../../../docs/specs/README.md) when what it does changed,
      [Formats](../../../docs/formats.md) when a route, the probe text, the report's JSON or a
      gRPC service name changed, an [ADR](../../../docs/adr/README.md) for a choice that is hard
      to reverse, and `breaking` on the PR if the public API or a format changed.
- [ ] Check the docs in the diff: doc comments are short and plain with an example where it
      helps, and never link a Markdown file; `docs/` pages use Mermaid for diagrams and tables
      over long lists; every relative link and `#anchor` resolves.
- [ ] Check each commit subject and the PR title against
      [Commit messages](../../../CONTRIBUTING.md#commit-messages): the type matches the change
      (`feat` only for something users can do now, `!` for a breaking change).
- [ ] Look for bugs: a lock held across `.await`, a check that can wait without a bound inside
      its own code, an error swallowed without a log, a status compared the wrong way
      round (`Status` orders from best to worst), and the patterns past reviews found (below).
- [ ] Look for what only shows at scale or during an outage: a std lock or a long loop on an
      async thread, a second poller for one registry, work done per probe request, and how many
      log lines and metric points each poll makes while a dependency is down.

## Bugs past reviews found

Each was real and passed CI in sisa-caching-rs, whose async code has the same shape. Look for the
same shape anywhere in a change.

- A flag released before a final recheck, with no ordering that makes the other side's writes
  visible. Pair the swaps with `AcqRel` and recheck after releasing.
- A last-owner check (`Arc::strong_count`) made outside the lock that hands out clones.
- A time read before waiting for a lock, then compared with what was written during the wait.
- Callers that wait on a shared operation and take its success for granted. Hand them its error,
  and let them redo it if its owner was cancelled.
- A future dropped halfway (a check's timeout, a `select!`) that leaves shared state behind: a
  held lock, a result never stored, a poller that never stops.
- A watermark that vouches for a run of finished items while an earlier one is still running.
- A span or log line on a helper another path also calls, or a log line per poll while a check
  stays down.
- A test that can't fail: a wait for a value the test never set, or a sleep standing in for proof
  that something happened. Break the code the test guards and watch it fail.

## Whole-repository review

- [ ] Start from `main`'s head and note its commit. Read
      [Guarantees and limits](../../../docs/architecture.md#guarantees-and-limits) and the
      [rules](../../../docs/architecture.md#rules-that-hold-the-design-together) first, then
      each crate.
- [ ] For each invariant below, check a test still guards it: break the code and see it fail.
      An invariant the docs promise with no test is a finding. Name the test in the table once
      it exists.
- [ ] Measure before calling something slow, in alternating rounds of the old and the new code;
      for a change under 5%, compare callgrind instruction counts instead.
- [ ] Write the findings as one document: severity, evidence, and the fix grouped into proposed
      PRs, each needing its own issue.

| Invariant | Test |
| --- | --- |
| One unhealthy check fails its probe; degraded doesn't | `one_unhealthy_check_fails_readiness`, `a_degraded_check_keeps_readiness` |
| `failure(Degraded)` reports an unhealthy check as degraded | `failure_degraded_reports_an_unhealthy_check_as_degraded` |
| A check past its timeout, or one that panics, fails without stopping the others | `a_check_past_its_timeout_fails`, `a_panicking_check_fails_without_stopping_the_others` |
| Before the first run, readiness fails and liveness passes | `before_the_first_run_readiness_fails_and_liveness_passes` |
| Draining fails readiness, never liveness | `draining_fails_readiness_but_not_liveness` |
| Stale results fail both probes | `stale_results_fail_both_probes` |
| Stale results fail the report | `a_stale_report_fails_and_says_why` |
| The probe text follows the Kubernetes API server | `a_passing_probe_says_ok`, `verbose_lists_every_line`, `a_failing_probe_lists_only_its_failures` |
| There is no `/healthz` | `there_is_no_healthz` |
| gRPC `Check` answers as the probes do, and an unknown service is `NOT_FOUND` | `a_failing_ready_check_stops_readiness_not_liveness`, `an_unknown_service_is_not_found` |

## Gotchas

- `Status` derives `Ord` from best to worst, so `max` is the worst status and `min` the best.
- `cargo +1.94 check` can fail where stable passes. Read the exit code, not a piped tail.
- Not bugs, by design: a check that hasn't run yet passes `/livez` and fails `/readyz`; a
  degraded check returns 200; `/health` returns 503 only when a check is unhealthy or its
  results are stale.

## Report

```markdown
**Blocking**
- `crates/core/src/registry.rs:120`: what is wrong. Fix: what would fix it.

**Not blocking**
- `file:line`: what is wrong. Fix: what would fix it.

Checks: all passed, or the failing check and its first error.
```

If nothing blocks, say so in one line instead of the Blocking section.
