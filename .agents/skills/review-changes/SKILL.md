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
      [rules](../../../docs/01-architecture.md#rules-that-hold-the-design-together), the
      [decisions](../../../docs/01-architecture.md#decisions) and the
      [code rules](../../../AGENTS.md#code) in AGENTS.md. The ones most often broken:
  - a probe or the report never runs a check: they read the poller's latest results;
  - a check shipped for `live` never touches a dependency;
  - a response never carries a result's error, only its description;
  - no new dependency in a library crate.
- [ ] Check what goes with the change: a test for each new behavior, doc comments on new public
      items, the `docs/` page it affects (above all the
      [formats](../../../docs/01-architecture.md#formats)), and `breaking` on the PR if the
      public API, a route, the probe text or the report's JSON changed.
- [ ] Check the docs in the diff: doc comments are short and plain with an example where it
      helps, and never link a Markdown file; `docs/` pages use Mermaid for diagrams and tables
      over long lists; every relative link and `#anchor` resolves.
- [ ] Check each commit subject and the PR title against
      [Commit messages](../../../CONTRIBUTING.md#commit-messages): the type matches the change
      (`feat` only for something users can do now, `!` for a breaking change).
- [ ] Look for bugs: a lock held across `.await`, a check that can wait without a bound inside
      its own code, an error swallowed without a log, and a status compared the wrong way
      round (`Status` orders from best to worst).
- [ ] Look for what only shows at scale or during an outage: a std lock or a long loop on an
      async thread, a second poller for one registry, work done per probe request, and how many
      log lines and metric points each poll makes while a dependency is down.

## Whole-repository review

- [ ] Start from `main`'s head and note its commit. Read
      [Guarantees and limits](../../../docs/01-architecture.md#guarantees-and-limits) and the
      rules first, then each crate.
- [ ] For each invariant below, check the test still guards it: break the code and see it fail.
      An invariant the docs promise with no test is a finding.
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
| The probe text follows the Kubernetes API server | `a_passing_probe_says_ok`, `verbose_lists_every_line`, `a_failing_probe_lists_only_its_failures` |
| There is no `/healthz` | `there_is_no_healthz` |

## Gotchas

- `Status` derives `Ord` from best to worst, so `max` is the worst status and `min` the best.
- `cargo +1.89 check` can fail where stable passes. Read the exit code, not a piped tail.
- Not bugs, by design: a check that hasn't run yet passes `/livez` and fails `/readyz`; a
  degraded check returns 200; `/health` returns 503 only when a check is unhealthy.

## Report

```markdown
**Blocking**
- `crates/core/src/registry.rs:120`: what is wrong. Fix: what would fix it.

**Not blocking**
- `file:line`: what is wrong. Fix: what would fix it.

Checks: all passed, or the failing check and its first error.
```

If nothing blocks, say so in one line instead of the Blocking section.
