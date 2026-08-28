# Portable rules

Stack-agnostic rule fragments, one concern per file. Extracted from a production
Rust + TypeScript monorepo and de-identified. Every rule here carries the incident
that produced it — that is deliberate. A bare prohibition gets rationalized away by
the next agent under deadline; a prohibition with a cost attached does not.

## Index

| Rule | Covers |
|---|---|
| [git-discipline.md](git-discipline.md) | Dead-branch liveness check, squash-merge detection, branch/push discipline, conventional commits, multi-agent git |
| [evidence-discipline.md](evidence-discipline.md) | Check the destination before trusting an absence; read runtime state, never guess it |
| [agent-parallelism.md](agent-parallelism.md) | Split by file count not concept; one worktree per agent; never two builds at once; seeding and cleanup |
| [timeouts.md](timeouts.md) | The 5-minute ceiling on every gate, and how to fit under it |
| [definition-of-done.md](definition-of-done.md) | End-to-end or not done; size is never a signal; blocks are routed around |
| [comments.md](comments.md) | What a comment must earn; never narrate the fix |
| [debugging-discipline.md](debugging-discipline.md) | Instrument before theorizing; revert before layering; find the regression commit |
| [token-efficiency.md](token-efficiency.md) | Targeted reads, mental cache, batched calls, minimal diffs |
| [testing-gates.md](testing-gates.md) | What actually enforces anything; process-per-test; sharding; un-hangable tests; complete-surface mocks |
| [layer-boundaries.md](layer-boundaries.md) | 4-layer direction as a test; ceilings not bans; how to open a god module |
| [dependency-hygiene.md](dependency-hygiene.md) | Before adding a package; no dead weight; the metric to watch |
| [error-channels.md](error-channels.md) | Two channels — user-actionable vs dev-only; never `console.error` |
| [event-streams.md](event-streams.md) | Activity vs Alerts, strictly separated |
| [ui-remote-states.md](ui-remote-states.md) | Never render a raw payload; plain-English copy; `ready` / `empty` / `unreachable` as a type |
| [data-over-binary.md](data-over-binary.md) | Fix customer behaviour in published data, not in the shipped binary |

## How to use them

These are instruction fragments, not Codex execution-policy `.rules` files.
Codex automatically loads `AGENTS.md` but does not expand Markdown `@` imports.

Use [`../gates/rust/render-agent-docs`](../gates/rust/render-agent-docs) to embed
a selected set into generated `AGENTS.md` files. Keep fragment names and overlay
in one manifest; run renderer after edits and `--check` in CI.

```toml
rules = ["git-discipline", "evidence-discipline", "timeouts"]
rules_dir = "rules"
overlay = "docs/agent-overlay.md"

[targets.agents]
path = "AGENTS.md"
title = "Project instructions"
```

```bash
render-agent-docs --manifest agent-docs.toml
render-agent-docs --manifest agent-docs.toml --check
```

For machine-wide personal guidance, place concise instructions directly in
`~/.codex/AGENTS.md`. Use `~/.codex/rules/*.rules` only for command execution
policy; those files use `prefix_rule(...)`, not Markdown.

## Picking a subset

Don't take all fifteen. Context is the budget.

- **Any repo, any stack:** `git-discipline`, `evidence-discipline`, `comments`,
  `definition-of-done`, `token-efficiency`.
- **Multi-agent work:** add `agent-parallelism`, `timeouts`.
- **Has a test suite and hooks:** add `testing-gates`.
- **Layered backend:** add `layer-boundaries`, `dependency-hygiene`.
- **Has a UI:** add `ui-remote-states`, `error-channels`, `event-streams`.
- **Engine + per-tenant data plane:** add `data-over-binary`.

## Adapting a rule

Each file names its own generics — `notifyUser` / `logDev`, "the trunk", "the task
tracker", "the build directory". Rename to your codebase's actual symbols on the
way in; a rule naming a function that does not exist gets ignored wholesale.

Keep the incidents. They are the load-bearing part.
