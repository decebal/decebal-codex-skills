# Token and Context Efficiency

Patterns for keeping Codex focused in large repositories.

## Search narrowly first

1. Search for exact symbols or filenames with `rg` and `rg --files`.
2. Read only relevant ranges before loading whole files.
3. Batch independent reads and checks.
4. Do not re-read unchanged files without a reason.

## Keep scope explicit

- Change only requested behavior.
- Keep unrelated user edits intact.
- Prefer small, reviewable diffs.
- Put acceptance conditions and verification commands in the task.
- Split parallel work by disjoint file ownership.

## Optimize `AGENTS.md`

Codex discovers `AGENTS.md` from repository root toward current directory and
combines applicable files. Keep each layer short and local:

```markdown
# Commands

task test
task lint

# Boundaries

- Use Bun for JavaScript dependencies.
- Run Rust tests sequentially.
- Never modify generated files by hand.
```

Deep instructions belong in a nested `AGENTS.md`, not in every parent file.

## Keep skills progressively disclosed

Codex uses skill metadata for discovery, then loads `SKILL.md` when selected.
Keep frontmatter descriptions precise. Move large examples and reference data to
`references/`, and load only references needed for current workflow.

## Keep hook output small

- Silent exit means no advisory context is added.
- Denials need one actionable reason.
- `additionalContext` should contain only information Codex needs next.
- Safe rewrites use `updatedInput` with `permissionDecision: "allow"`.
- Put durable policy in `AGENTS.md` or `.codex/rules/*.rules`, not repeated hook
  messages.

`codex-guard` includes payload-size tests so hook messages cannot grow unnoticed.

## Size tasks for resumption

Each task should have one concern, explicit file scope, self-contained acceptance
criteria, and exact quality gates. A fresh context should be able to finish it
from task text plus repository state.

## Tune model output in `config.toml`

```toml
model_reasoning_effort = "medium"
model_verbosity = "medium"
```

Raise reasoning effort for complex debugging or architecture. Lower verbosity
for routine execution when detailed narration adds no value.
