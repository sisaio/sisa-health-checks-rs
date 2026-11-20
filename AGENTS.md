# Agent instructions

For coding agents working in this repository. People read [CONTRIBUTING.md](CONTRIBUTING.md)
first; this file adds what an agent needs on top, and every sisaio repository has the same
sections in the same order.

## What this is

sisa-health-checks: Rust crates that tell Kubernetes, load balancers and people whether a service
is working: a registry that runs checks in the background, `/livez` and `/readyz` probes and a
report over HTTP or gRPC, and checks for PostgreSQL, Redis and NATS. 0.0.1 is on crates.io; the
[roadmap](ROADMAP.md) says what comes next.

## Where things are

| You need | Read |
| --- | --- |
| How the crates fit, the guarantees and the rules | [docs/architecture.md](docs/architecture.md) |
| What one crate does | Its page in [docs/specs/](docs/specs/README.md) |
| The probe text, the report and the gRPC answers | [docs/formats.md](docs/formats.md) |
| A poll or a probe step by step | [docs/flows.md](docs/flows.md) |
| The metrics and the log lines | [docs/observability.md](docs/observability.md) |
| Why something was chosen | [docs/adr/](docs/adr/README.md) |
| The checks, tests, code style, labels and Git rules | [CONTRIBUTING.md](CONTRIBUTING.md) |
| Labels, milestones, rulesets and the release setup | [.github/scripts/README.md](.github/scripts/README.md) |
| What is planned | [ROADMAP.md](ROADMAP.md) and the milestones |

## Commands

Run the checks in [Build and check](CONTRIBUTING.md#build-and-check) before you push, as CI runs
them, and check exit codes rather than piped output. The PostgreSQL, Redis and NATS tests need
Docker. Turn the hooks on once per clone:

```sh
git config core.hooksPath .githooks
```

## Hard rules

Breaking one needs a new [ADR](docs/adr/README.md) first.

- There is no `/healthz` and no `/startupz`
  ([ADR-0001](docs/adr/0001-probes-follow-the-kubernetes-api-server.md)).
- Probes and the report never run a check. Only the poller, through `HealthRegistry::run`, does;
  everything else reads the results it stored.
- Follow the [rules that hold the design together](docs/architecture.md#rules-that-hold-the-design-together).
- A check owns no thread, task or timer of its own, and never retries: the registry bounds it with
  its timeout and runs it again on the next poll.
- A check shipped for `live` touches nothing outside the process. Every check of a dependency is
  tagged `ready` only.
- A check never puts a credential, a connection string or a query in its description or details:
  `/health` shows both. The error goes through `with_error`, which only the logs see.
- Don't add dependencies to the library crates without asking. A new check for another system is
  its own crate, `sisa-health-checks-<system>`, with a feature of the same name on the facade.
  Ask before adding one.
- The maintainer picks public names. Use given names exactly, and propose new ones instead of
  inventing them.
- Keep code minimal, readable, and working; the smallest change that does the job wins.
- A performance change shows its numbers before and after, measured in alternating rounds of
  the old and the new code, and with instruction counts when the difference is under 5%.
- API docs go in Rust doc comments, not in Markdown files linked from them.

## How to work

1. **Issue first.** Never open a PR without an issue. Work on an issue the maintainer assigned
   or named; when the maintainer asks for work that has none, open it first with the matching
   form's sections, its type (Bug, Feature or Task), its labels and the milestone of the next
   release, and assign it to the maintainer.
2. **Branch** from `main` as `<type>/<issue>-<topic>`, as [Branches](CONTRIBUTING.md#branches)
   says. Use another name only when the maintainer gives one. Rebase onto `main` to update a
   branch; never merge `main` into it.
3. **Docs with the code.** A change to what a crate does updates its
   [spec](docs/specs/README.md), a change to a data format updates
   [docs/formats.md](docs/formats.md), and a choice that is hard to reverse gets an
   [ADR](docs/adr/README.md), all in the same PR.
4. **Commits and the PR title** follow [Commit messages](CONTRIBUTING.md#commit-messages):
   `type(scope): summary`. Commits carry the contributor's own name as author and committer. Add
   no co-author or tool trailers, and don't mention AI tools in commits, PR titles, PR
   descriptions or GitHub comments.
5. **Labels and milestone.** Label every issue and PR as
   [Issues, labels and milestones](CONTRIBUTING.md#issues-labels-and-milestones) says. The PR
   says `Closes #<n>`, carries its `type:` label and the issue's `area:` labels, and has its
   author as assignee. Labels, milestones and rulesets come from files in `.github/`: propose a
   change there instead of making one on GitHub.
6. **The PR template.** Fill in the [PR template](.github/pull_request_template.md): What
   changes gives a Before and an After, and Evidence shows the commands you ran and what they
   printed.
7. **Show before pushing.** When asked to show or evaluate a change, present the diff for review
   before pushing.

## Settled decisions

Don't re-propose an accepted [ADR](docs/adr/README.md) without new evidence. To change one, write
a new ADR that supersedes it.

## Skills

Step-by-step procedures follow the [Agent Skills](https://agentskills.io/specification) format,
one folder per skill in `.agents/skills/`. `.claude/skills/` links to the same folders, so tools
that look in either place find them. Read the matching one before starting:

| Skill | Use it to |
| --- | --- |
| [review-changes](.agents/skills/review-changes/SKILL.md) | Review a diff, PR or docs page against the checks, rules and decisions |
| [release](.agents/skills/release/SKILL.md) | Prepare the release PR or find why a release failed |
| [triage](.agents/skills/triage/SKILL.md) | Triage new issues: duplicates, reproduction, labels, milestone |
| [write-adr](.agents/skills/write-adr/SKILL.md) | Record a decision as a numbered ADR |
| [write-spec](.agents/skills/write-spec/SKILL.md) | Write or update a crate's spec or a flow |
