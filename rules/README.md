# Portable rules

Stack-agnostic rule fragments, one concern per file. Extracted from a production
Rust + TypeScript monorepo and de-identified. Every rule here carries the incident
that produced it — that is deliberate. A bare prohibition gets rationalized away by
the next agent under deadline; a prohibition with a cost attached does not.

These are Markdown instruction fragments, not Codex execution-policy `.rules`
files.

## Index

Many of these rules are **path-scoped**: `codex-guard rule-scope` delivers each
one the first time a session touches a matching file, so it costs nothing before
then. See [How rules reach Codex](#how-rules-reach-codex) below.

| Rule | Covers | Scope |
|---|---|---|
| [git-discipline.md](git-discipline.md) | Dead-branch liveness check, one open PR per session, squash-merge detection, branch/push discipline, conventional commits, multi-agent git | unconditional |
| [evidence-discipline.md](evidence-discipline.md) | Check the destination before trusting an absence; read runtime state, never guess it | unconditional |
| [agent-parallelism.md](agent-parallelism.md) | Split by file count not concept; one worktree per agent; never two builds at once | unconditional (trimmed) |
| [timeouts.md](timeouts.md) | The 5-minute ceiling on every gate; ask which phase a slow step runs in before making it faster | unconditional (trimmed) |
| [process-ownership.md](process-ownership.md) | Every child has a parent that reaps it; bounded/cached/reaped hot-path spawns | unconditional (trimmed) |
| [definition-of-done.md](definition-of-done.md) | End-to-end or not done; size is never a signal; blocks are routed around | unconditional |
| [estimation.md](estimation.md) | Never estimate in time; rock/sand/water is confidence, not size | unconditional (trimmed) |
| [comments.md](comments.md) | What a comment must earn; never narrate the fix | unconditional |
| [anti-slop.md](anti-slop.md) | Report findings, don't perform them: no escalation openers, X-not-Y frames, significance labels | unconditional |
| [debugging-discipline.md](debugging-discipline.md) | Instrument before theorizing; revert before layering; find the regression commit | unconditional |
| [token-efficiency.md](token-efficiency.md) | Targeted reads, mental cache, batched calls, minimal diffs | unconditional |
| [documents-not-artifacts.md](documents-not-artifacts.md) | Deliverables are versioned, indexed documents under `docs/`; never hosted pages | unconditional |
| [pr-evidence-report.md](pr-evidence-report.md) | The HTML report a PR ships: sections, screenshots, before/after, copy blocks | `docs/**/*.html`, `**/.github/pull_request_template.md` |
| [testing-gates.md](testing-gates.md) | What actually enforces anything; a source scan must not be a compiled test | unconditional (trimmed) |
| [testing-authoring.md](testing-authoring.md) | Process-per-test, sharding, test file conventions, un-hangable tests, module mocks | test files, gates, workflows |
| [layer-boundaries.md](layer-boundaries.md) | 4-layer direction as a test; ceilings not bans; how to open a god module | `**/*.rs`, `**/Cargo.toml`, `**/src/**/*.{ts,tsx,svelte}` |
| [dependency-hygiene.md](dependency-hygiene.md) | Before adding a package; no dead weight; the metric to watch | `**/Cargo.toml`, `**/Cargo.lock`, `**/package.json`, `**/bun.lock` |
| [error-channels.md](error-channels.md) | Two channels — user-actionable vs dev-only; never `console.error` | `**/*.{svelte,ts,tsx,js,jsx}` |
| [event-streams.md](event-streams.md) | Activity vs Alerts, strictly separated | `**/*.{svelte,ts,tsx,js,jsx}` |
| [ui-remote-states.md](ui-remote-states.md) | Never render a raw payload; plain-English copy; `ready` / `empty` / `unreachable` as a type | `**/*.{svelte,ts,tsx,js,jsx}` |
| [data-over-binary.md](data-over-binary.md) | Fix customer behaviour in published data, not in the shipped binary | unconditional (trimmed) |

**Detail and evidence:** incident narratives, measurements and code samples live
in [`docs/rules-reference/`](../docs/rules-reference/), installed to
`~/.codex/rules-reference/`. A rule names the file to read when it needs one.

## How rules reach Codex

Codex reads `AGENTS.md` files and never resolves an `@` import, so a line naming a
rule file loads nothing. A rule reaches a session through one of three channels:

| Channel | Carries | Mechanism |
|---|---|---|
| Rendered `AGENTS.md` | rules with no `paths:` line | [`render-agent-docs`](../gates/rust/render-agent-docs) inlines each body into the target file and leaves `paths:`-scoped rules out |
| `codex-guard rule-scope` | rules with `paths:` frontmatter | delivers a rule the first time a session touches a file matching one of its globs |
| On demand | `~/.codex/rules-reference/*.md` | the agent reads the file a rule's pointer names |

Rules about editing a kind of file are scoped. Rules about behaviour (git,
reporting, estimating, gate failures, spawning agents) stay unconditional,
because no file touch triggers them. `rule-scope` reads the globs from the
`paths:` block, so edit a glob only to change what the rule covers.

The two instruction budgets differ
([Codex `AGENTS.md` docs](https://developers.openai.com/codex/agent-configuration/agents-md)):

- **`~/.codex/AGENTS.md`**, the global file, is read whole, with no cap.
- **A repo's `AGENTS.md` chain**, from the repo root down to the working
  directory, shares `project_doc_max_bytes`: 32 KiB by default. Codex cuts the
  text at that byte and only logs a warning, so a rule past the cut is gone and
  nothing in the session shows it.

## How to use them

Three placement options:

| Where | Loads for | Use when |
|---|---|---|
| `~/.codex/agent-rules/`, rendered into `~/.codex/AGENTS.md` | every project on the machine | the rule is you, not the repo (git discipline, evidence discipline, comments) |
| `.agents/rules/` in the repo, rendered into the repo's `AGENTS.md` | everyone on the repo, agents included | the rule is the repo's (layer boundaries, testing gates) |

`codex-guard rule-scope` reads path-scoped rules from `~/.codex/agent-rules/`,
a repo's `.agents/rules/` and its `.claude/rules/`, and nowhere else. A
path-scoped rule rendered from any other directory is inlined into `AGENTS.md`,
with a warning, because nothing would deliver it on demand.
| Hand-written in `AGENTS.md` | — | never; it drifts from its source, and in a repo it spends the 32 KiB budget |

Install the machine-wide set without this index, and the reference docs beside
it:

```bash
diff -rq --exclude README.md rules/ ~/.codex/agent-rules/   # an installed copy you edited by hand shows here; keep what it adds
rsync -a --exclude README.md rules/ ~/.codex/agent-rules/
rsync -a docs/rules-reference/ ~/.codex/rules-reference/
```

**Never copy these into `~/.codex/rules/`.** That directory holds Codex's
execution-policy `.rules` files (`prefix_rule(...)`), and Markdown does not
belong in it.

Render the global file from a manifest in `~/.codex/agent-docs.toml`. `~/`
expands in `rules_dir`, `overlay` and target paths; other relative paths
resolve from the directory the renderer runs in. Values are quoted strings:

```toml
rules     = ["git-discipline", "evidence-discipline", "definition-of-done", "comments", "anti-slop"]
rules_dir = "~/.codex/agent-rules"
overlay   = "~/.codex/agents-overlay.md"   # your own machine-level instructions

[targets.global]
path      = "~/.codex/AGENTS.md"
title     = "Personal instructions"
max_bytes = "0"
```

```bash
render-agent-docs --manifest ~/.codex/agent-docs.toml
```

`max_bytes = "0"` lifts the renderer's size cap for this target. Only the global
file gets it, because Codex reads that file whole; a repo target keeps a cap so
its chain stays inside 32 KiB. A repo renders the same way from its own manifest
([`gates/examples/agent-docs.toml`](../gates/examples/agent-docs.toml)) and runs
`render-agent-docs --check` in pre-push, so a hand-edited `AGENTS.md` fails the
push.

Path-scoped rules need the hook: register `codex-guard rule-scope` in
`~/.codex/hooks.json` ([`hooks/README.md`](../hooks/README.md)).

## Measuring

```bash
wc -c ~/.codex/AGENTS.md                             # no cap, but every byte loads in every session
wc -c AGENTS.md <subdir>/AGENTS.md                   # one repo chain, root to working directory: total under 32768
grep -L "^paths:" ~/.codex/agent-rules/*.md          # rules eligible to render (only those in the manifest's list do)
grep -l "^paths:" ~/.codex/agent-rules/*.md          # the rules rule-scope delivers
```

## Picking a subset

Don't take all twenty-one. Context is the budget: every unconditional rule is
paid for in every session, and a repo chain has 32 KiB for everything.

- **Any repo, any stack:** `git-discipline`, `evidence-discipline`, `comments`,
  `anti-slop`, `definition-of-done`, `estimation`, `token-efficiency`,
  `documents-not-artifacts`.
- **Multi-agent work:** add `agent-parallelism`, `timeouts`, `process-ownership`.
- **Anything that shells out on a hot path** (statusline, hook, watcher, poller),
  or drives a browser/emulator/container from an agent: add `process-ownership`.
- **Has a test suite and hooks:** add `testing-gates` and `testing-authoring`.
- **Layered backend:** add `layer-boundaries`, `dependency-hygiene`.
- **Has a UI:** add `ui-remote-states`, `error-channels`, `event-streams`.
- **Ships user-visible change through PRs:** add `pr-evidence-report` — the
  report is the only place a reviewer sees the feature work, since no gate
  demonstrates behaviour.
- **Engine + per-tenant data plane:** add `data-over-binary`.

## Adapting a rule

Each file names its own generics — `notifyUser` / `logDev`, "the trunk", "the task
tracker", "the build directory". Rename to your codebase's actual symbols on the
way in; a rule naming a function that does not exist gets ignored wholesale.

Keep the incidents. They are the load-bearing part. When one grows past a few
lines, move it to `docs/rules-reference/` and leave the cost in the rule.
