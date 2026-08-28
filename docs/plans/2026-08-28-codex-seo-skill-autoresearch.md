# Codex SEO and Skill Autoresearch Design

Date: 2026-08-28

## Decision

Add three Codex skills:

1. `codex-seo`, a small interoperability coordinator for installed SEO skills
   with an evidence-led fallback.
2. `skill-autoresearch`, a bounded skill-improvement protocol backed by a
   self-contained Rust runner.
3. `app-store-optimization`, an MIT-compatible Codex port with current
   Apple/Google metadata rules and a Rust validator.

Do not vendor or adapt files from
[`AgriciDaniel/codex-seo`](https://github.com/AgriciDaniel/codex-seo). Its
repository `LICENSE` at revision `97c59bcdac3c9538bf0e3ae456c1e73aa387f85a`
prohibits redistribution and distributed derivative works. Its plugin manifest
claims MIT, but the root license controls this repository's copying decision.

## Codex SEO boundary

`codex-seo` owns routing, evidence rules, and fallback output. It may invoke an
already-installed `$seo` or specialist `$seo-*` skill by public skill name. It
does not include upstream prompts, agents, scripts, schemas, hooks, or API
wrappers.

When no specialist is available, the coordinator performs only checks supported
by current browser, web, repository, or user-provided evidence. Unmeasured areas
are marked `not checked` or `setup required`; they never become inferred scores.

## Autoresearch boundary

`skill-autoresearch` separates four concerns:

- Codex chooses one focused skill mutation.
- A frozen evaluator emits one finite numeric score.
- Rust executes the evaluator without a shell, applies timeout and iteration
  limits, records `results.tsv`, and decides `keep` or `discard`.
- Git preserves experiments. Codex commits a candidate before measuring it and
  uses `git revert` for discarded candidates. The Rust runner never commits,
  resets, restores, pushes, or edits the target skill.

Strict improvement wins. Equal scores discard. If simplicity matters, encode a
complexity penalty in the frozen metric before recording the baseline.

## Runtime contract

State lives in a caller-chosen directory outside the installed skill. `init`
records target skill, metric direction, candidate cap, and timeout. `baseline`
must run once. Each `candidate` accepts either `--score NUMBER` or an executable
argument vector after `--`. Evaluator output ends with either a bare number or
`score=NUMBER`.

The runner keeps:

- `config.tsv`: immutable experiment contract;
- `results.tsv`: baseline and candidate ledger;
- `last-evaluator.stdout` and `last-evaluator.stderr`: latest raw evaluator
  evidence.

Evaluator errors and timeouts consume an iteration and are recorded. They do not
replace the incumbent.

## App Store Optimization boundary

Port useful workflow ideas from
[`alirezarezvani/claude-code-aso-skill`](https://github.com/alirezarezvani/claude-code-aso-skill)
revision `94148561f173a917b45f8fd125e3025fa25cba85` under its MIT license. Retain
license and modification notice inside installable skill.

Rewrite rather than copy its long prompt and Python package. Main corrections:

- Google Play app name is 30 characters, not upstream skill's stale 50.
- Apple keyword field is 100 UTF-8 bytes, not 100 characters.
- Search volume, rank, download, and conversion claims require real data; sample
  numbers are never presented as observations.
- Rust `aso-lint` validates listing metadata and evaluates two-proportion
  experiments. Existing `$codex-seo`, `$seo-*`, and `$skill-autoresearch` skills
  handle research, content, images, and iterative optimization.

## Verification

- Official Codex `quick_validate.py` validates both skills.
- Repository `skill-metadata-check` validates all metadata and plugin discovery.
- Rust format, Clippy, unit tests, and CLI smoke tests validate the runner.
- Rust boundary and experiment tests validate `aso-lint`.
- Existing repository gate and skill test suites remain green.
