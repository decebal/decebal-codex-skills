---
name: skill-autoresearch
description: "Improve a Codex skill through bounded autoresearch: freeze a mechanical evaluator, record a baseline, make one focused change per iteration, keep only measured improvements, and preserve experiment evidence. Use when asked to autoresearch, optimize, tune, or systematically improve a SKILL.md, skill runtime, trigger boundary, or skill eval. The bundled runner is written in Rust."
---

# Skill Autoresearch

Run controlled experiments against one Codex skill. Optimize measured behavior,
not prose aesthetics.

## Preconditions

1. Read target `SKILL.md`, its directly referenced files, and applicable
   `AGENTS.md` instructions.
2. Define target behavior and failure examples.
3. Build or select a mechanical evaluator. Freeze evaluator, fixtures, metric,
   model/version if applicable, and command before baseline.
4. Work on a dedicated branch or clean worktree. Limit edits to target skill and
   predeclared eval files.

Read [references/designing-skill-evals.md](references/designing-skill-evals.md)
before creating a new evaluator.

## Locate runner

Runner ships with this skill at `scripts/Cargo.toml`. Resolve that path relative
to installed `SKILL.md`, then use its absolute path:

```bash
cargo run --quiet --locked --manifest-path <skill-autoresearch-dir>/scripts/Cargo.toml -- --help
```

It has no third-party Rust dependencies. It never edits target skill or invokes
Git. Rust and Cargo are required.

## Initialize

Choose untracked state directory, usually `.autoresearch/<skill-name>`:

```bash
cargo run --quiet --locked --manifest-path <runner-manifest> -- init \
  .autoresearch/<skill-name> \
  --skill-path skills/<skill-name> \
  --metric pass-rate-minus-complexity \
  --direction higher \
  --max-iterations 12 \
  --timeout-seconds 300
```

Record baseline using frozen evaluator. Runner executes argument vector after
`--` directly, without shell expansion:

```bash
cargo run --quiet --locked --manifest-path <runner-manifest> -- baseline \
  .autoresearch/<skill-name> \
  --commit <baseline-commit> \
  --description "frozen baseline" \
  -- cargo run --quiet --manifest-path tooling/skill-eval/Cargo.toml -- <args>
```

Evaluator's final non-empty stdout line must be a finite number or
`score=<number>`. When evaluation happens elsewhere, replace command with
`--score <number>`.

## Experiment loop

Repeat until candidate cap, goal, or honest plateau:

1. Inspect current incumbent and one concrete failing eval.
2. Form one hypothesis.
3. Make one focused mutation. Do not change frozen evaluator or metric.
4. Run structural validation and relevant tests.
5. Commit candidate with normal descriptive commit message.
6. Measure it:

```bash
cargo run --quiet --locked --manifest-path <runner-manifest> -- candidate \
  .autoresearch/<skill-name> \
  --commit <candidate-commit> \
  --description "one-line hypothesis" \
  -- <frozen-evaluator> <args>
```

7. Keep `verdict=keep`. For `discard`, `error`, or `timeout`, preserve history
   with `git revert --no-edit <candidate-commit>` before next mutation.
8. Read `results.tsv` and raw `last-evaluator.*` evidence. Never infer success
   from command completion alone.

Strict improvement wins. Equal score discards. Encode simplicity, latency, or
cost penalties in frozen metric if they should break ties.

## Stop rules

Stop when any condition holds:

- candidate cap reached;
- acceptance threshold reached and held-out failures are resolved;
- three consecutive focused candidates fail to improve;
- evaluator is invalid, contaminated, or no longer measures requested behavior;
- further work needs user authority or a changed scope.

Do not push, publish, deploy, install over an existing skill, or rewrite shared
history unless user separately asks. Finish with incumbent commit, score delta,
kept/discarded counts, unresolved evals, and exact verification commands.
