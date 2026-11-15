---
name: ritual-full
description: "The ritual flow (Think, Spec, Plan, Build, Review, Ship, Reflect on a Chronis epic) with the full toolset required: allstack recall and check receipts, a Semantic ISA section in every design, two independent fresh-context design reviewers, the repo's prove-it on every Review (forged when missing), cos hires per step, graphify context, codex-guard hooks. Runs a tool preflight first and stops on anything missing. EGM stays off unless asked for. Triggers on: ritual full, full ritual, full flow, plan this feature with every tool, ritual with semantic isa, ritual with egm."
---

# ritual-full

The same flow as [ritual](../ritual/SKILL.md): same steps, same files, same `ritual-*` lines on
the epic, same gates. Read that skill first; this one states only what changes. It also uses
[semantic-isa](../semantic-isa/SKILL.md). The preflight tools below are required. A missing hard
row blocks the flow until it is installed or the user says "continue without <tool>"; never skip
one silently. EGM is off here too: it runs only when the user asks for EGM
([ritual's opt-in table](../ritual/SKILL.md#opt-in-egm)). Install and a worked example:
[ritual, zero to hero](https://github.com/decebal/decebal-claude-skills/blob/main/docs/ritual-zero-to-hero.md).

## Preflight (first thing, every flow)

Run `ritual-feedback preflight --full --json`. It caches each spawning check (`cn --help`,
`allstack doctor`, `graphify --help`, `cos ritual check --help`) in `~/.ritual/preflight.json`
against the binary's path, size and mtime; only `ok` results are reused, file checks run live.
Pass `--recheck` when the user asks for a fresh check. When a tool fails in real use, run
`ritual-feedback preflight --full --stale <tool>` (`cn`, `allstack`, `graphify` or `cos`): it
rechecks that tool.

The binary's rows `cn`, `allstack`, `graphify`, `feedback` and `cos` apply as reported. Its
`semantic-isa`, `claude-guard` and `skills` rows read `~/.claude/` paths: ignore them on Codex and
check the `skills` and `codex-guard` rows below yourself (file reads, no process). Because those
rows can fail it, decide from the JSON rows, not the exit code. Stop if any hard row below is
`missing`. Where the binary's install hint differs from the table, use the table.

| Tool | Row | Check | Install when missing |
|---|---|---|---|
| Chronis `cn` | hard | `cn --help` exits 0; `.chronis/` in the main checkout | `cargo install chronis`, then `cn init` in the repo |
| allstack | hard | `allstack doctor` exits 0 and prints `"executables_ready": true`; `setup_ready: false` is reported, not blocking (the ritual never runs `allstack init`) | clone `all-source-os/allstack`, run `cargo install --path . --locked` in it |
| skills | hard | `ritual`, `semantic-isa`, `pr-proof`, `systematic-debugging`, `forge-prove-it` under `~/.agents/skills/` or the repo's `.agents/skills/` | `cp -R skills/<name> ~/.agents/skills/<name>` from this repo |
| graphify | hard | `graphify --help` exits 0; `graphify-out/` in the repo, else build it | `uv tool install graphifyy` then `graphify codex install`; the same reinstall fixes `graphify --help` exiting 126 with `bad interpreter` |
| codex-guard | hard | `command -v codex-guard` finds it (it has no `--help`; with no subcommand it prints usage and exits 2); the trusted Codex hooks config registers `codex-guard infra-guard`, `bash-hygiene` and `pr-guard` | `cargo install --path gates/rust/codex-guard` from this repo, then register the hooks as `configs/hooks.json` shows and trust them |
| ritual-feedback | hard | the preflight runs at all (it is this binary) | `cargo install --locked --git https://github.com/decebal/decebal-claude-skills ritual-feedback` |
| cos | soft | `cos ritual check --help` exits 0; missing prints the install command and does not stop: hires become `spawn_agent` agents in fresh contexts, approval takes the no-`cos` path in [spec.md](../ritual/references/spec.md#approve) | clone `decebal/chief-of-staff-v2`, run `task setup` in it |

Report: `preflight: cn ok · allstack ok · skills ok · graphify ok · codex-guard ok · feedback ok · cos ok · 4 cached, 0 checked`,
with `missing (<install command>)` in place of `ok` where it applies; the counts are the JSON's
`cached` and `checked`.

## What is on, per step

| Step | On by default | Hands over (in addition to ritual's) |
|---|---|---|
| Think | brief in conversation; `allstack recall` on the idea's topic; `graphify query` on the modules the idea touches | recall hits and graph nodes cited in the brief |
| Spec | native draft; a **Semantic ISA** section; two fresh-context reviewer agents (`spawn_agent`); `cos ritual check` when `cos` is installed | design with ISA section; two review files |
| Plan | one bead per ISA sink whose done-when is its VERIFY test; graph read back ([graph.md](../ritual/references/graph.md)) | plan with the ISA test matrix as trophy rows |
| Build | `cos agents spawn --role builder` (or a `spawn_agent` agent) per ready bead, own worktree; gates as `allstack check` receipts; codex-guard hooks on | ISA matrix rows as real tests |
| Review | a fresh-context reviewer runs the repo's `$prove-it`, forged first if missing ([review.md](../ritual/references/review.md)); sink-to-VERIFY check | code review naming each sink's VERIFY at file:line |
| Ship | `$pr-proof` evidence and `allstack check` receipts; one PR, one commit ([ship.md](../ritual/references/ship.md)) | the evidence report |
| Reflect | drift compared by hand on the design; new-sink check; `allstack remember --kind review` on the report | report under `docs/reports/` |

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

Two reviewer agents, each started with `spawn_agent` in a fresh context with only the design path
and the repo root, never see each other's output. Each writes its own review file with a
`verdict:` line (format in [spec.md](../ritual/references/spec.md)); a hired `ritual-goldfish` may
be one of the two. Approval needs both clean against the same design digest; point the approval
(`cos ritual approve --review`, or the `ritual-approved:` line without `cos`) at the later one and
name the other in the design's review history. Resolve findings, then re-run both. The approval
is the user's: in their terminal, or, when they tell you to approve in the conversation, with
`--yes` and their instruction quoted in your report ([spec.md](../ritual/references/spec.md#approve)).
Agents never approve on their own initiative.

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
Think and Spec, recording feedback with `--skill ritual-full`
([Feedback](../ritual/SKILL.md#feedback)), and per-org notes
([repo-notes.md](../ritual/references/repo-notes.md)) are as in [ritual](../ritual/SKILL.md).

Ported from [decebal-claude-skills](https://github.com/decebal/decebal-claude-skills/tree/main/skills/ritual-full).
