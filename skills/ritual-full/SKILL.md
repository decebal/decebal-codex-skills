---
name: ritual-full
description: "The ritual flow (Think, Spec, Plan, Build, Review, Ship, Reflect on a Chronis epic) with every tool on by default: EGM for brainstorm, draft, review and drift, run by the agent; a Semantic ISA section in every design; two independent fresh-context reviewers; cos hires per step; graphify context; codex-guard hooks. Runs a tool preflight first and stops on anything missing. Triggers on: ritual full, full ritual, full flow, plan this feature with every tool, ritual with egm, ritual with semantic isa."
---

# ritual-full

The same flow as [ritual](../ritual/SKILL.md): same steps, same files, same `ritual-*` lines on
the epic, same gates. Read that skill first; this one states only what changes. It also uses
[semantic-isa](../semantic-isa/SKILL.md). Every tool below is **on by default**. A missing tool
blocks the flow until it is installed or the user says "continue without <tool>"; never skip one
silently. Install and a worked example:
[ritual, zero to hero](https://github.com/decebal/decebal-claude-skills/blob/main/docs/ritual-zero-to-hero.md).

## Preflight (first thing, every flow)

Run each check, print one table, then stop if any row is `missing`:

| Tool | Check | Install when missing |
|---|---|---|
| Chronis `cn` | `cn --help` exits 0; `.chronis/` in the main checkout | `cargo install chronis`, then `cn init` in the repo |
| EGM | `~/.codex/config.toml` has `[plugins."egm@egm"] enabled = true` | add the marketplace `https://github.com/google/elephant-goldfish-model` and enable `egm@egm` as EGM's `docs/codex.md` describes, new session |
| skills | `ritual`, `semantic-isa`, `pr-proof`, `systematic-debugging` under `~/.agents/skills/` or the repo's `.agents/skills/` | `cp -R skills/<name> ~/.agents/skills/<name>` from this repo |
| cos | `cos ritual check --help` exits 0 | clone `decebal/chief-of-staff-v2`, run `task setup` in it |
| graphify | `graphify --help` exits 0; `graphify-out/` in the repo, else build it | `uv tool install graphifyy` then `graphify codex install`; the same reinstall fixes `graphify --help` exiting 126 with `bad interpreter` |
| codex-guard | `command -v codex-guard` finds it (it has no `--help`; with no subcommand it prints usage and exits 2); the trusted Codex hooks config registers `codex-guard infra-guard`, `bash-hygiene` and `pr-guard` | `cargo install --path gates/rust/codex-guard` from this repo, then register the hooks as `configs/hooks.json` shows and trust them |

Report: `preflight: cn ok · egm ok · skills ok · cos ok · graphify ok · codex-guard ok`, with
`missing (<install command>)` in place of `ok` where it applies. You run every EGM step below
yourself, spawning its reviewers with `spawn_agent` ([ritual's EGM section](../ritual/SKILL.md#egm)).

## What is on, per step

| Step | On by default | Hands over (in addition to ritual's) |
|---|---|---|
| Think | EGM `egm-brainstorm`; `graphify query` on the modules the idea touches | `concepts_brief.md` moved to the dated brief path; graph nodes cited |
| Spec | EGM `egm-draft`; a **Semantic ISA** section; review by EGM `egm-review` **and** a hired `ritual-goldfish`; resolve with EGM `egm-resolve` or `egm-review-loop`; `cos ritual check` | design with ISA section; two review files |
| Plan | one bead per ISA sink whose done-when is its VERIFY test; graph read back ([graph.md](../ritual/references/graph.md)) | plan with the ISA test matrix as trophy rows |
| Build | `cos agents spawn --role builder` per ready bead, own worktree; codex-guard hooks on | ISA matrix rows as real tests |
| Review | `cos agents spawn --role reviewer`; sink-to-VERIFY check | code review naming each sink's VERIFY at file:line |
| Ship | `$pr-proof` evidence; one PR, one commit ([ship.md](../ritual/references/ship.md)) | the evidence report |
| Reflect | EGM `egm-drift-report` on the design; new-sink check | report under `docs/reports/` |

## Spec: the Semantic ISA section

Every design gets a `## Semantic ISA` section, written with semantic-isa
([SKILL.md](../semantic-isa/SKILL.md)) and scaled to the change:

1. **Side-effect inventory:** every file write, network call, API, message, deploy and data write
   the change adds or touches. One row each, with reversibility.
2. **Instructions** in the five cores, each with a governance property and a typed binding
   (closed `input_schema` and `output_schema`).
3. **Taint:** sources (user input, fetched content, file reads, model output) and **sinks**.
4. **VERIFY before every sink:** a stated, deterministic check. "The model judged it safe" is not
   one.
5. **Violations** as recoverable exceptions: evidence recorded and the recovery path.
6. **ISA test matrix:** per sink, tainted input refused and VERIFY-cleared input admitted.

A change with no side effects says so in one line with the evidence (`no file, network, API,
message, deploy or data write: <the diff's scope>`). The Goldfish reviewers check this section
like any other; an inventory that misses a side effect visible in the code is a blocking finding.

## Spec: two reviewers, one approval

The two reviews run in separate fresh contexts and never see each other's output. Write each as
its own review file (format in [spec.md](../ritual/references/spec.md)). Approval needs both clean
against the same design digest; point `cos ritual approve --review` at the later one and name the
other in the design's review history. Resolve findings, then re-run both. The approval is the
user's: `cos ritual approve` in their terminal, or, when they tell you to approve in the
conversation, the same command with `--yes` and their instruction quoted in your report
([spec.md](../ritual/references/spec.md#approve)). Agents never approve on their own initiative.

## Review: sink-to-VERIFY check

For each sink in the design's ISA table, the reviewer finds the VERIFY in code and its test:
`rg -n '<sink call>'` lists every call site, and each one is reached only through the named check.
A sink with no VERIFY, or a call site that bypasses it, is a blocking finding. A sink in the code
that the design does not list is drift: back to Spec.

## Reflect: new sinks

Diff the shipped code's side effects against the design's inventory. A new sink, or one whose
VERIFY changed, goes in the report's Drift section and in the design, in the same change.

## Everything else

Finding the step, the `ritual-*` lines, where the epic and files live, the gates, bugs skipping
Think and Spec, and per-org notes ([repo-notes.md](../ritual/references/repo-notes.md)) are as in
[ritual](../ritual/SKILL.md).

Ported from [decebal-claude-skills](https://github.com/decebal/decebal-claude-skills/tree/main/skills/ritual-full).
