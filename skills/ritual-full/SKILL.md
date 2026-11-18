---
name: ritual-full
description: "The ritual flow (Think, Spec, Plan, Build, Review, Ship, Reflect on a Chronis epic) with the full toolset required: allstack recall and check receipts, a Semantic ISA section in every design, two independent fresh-context design reviewers, the repo's prove-it on every Review (forged when missing), cos hires per step, graphify context, codex-guard hooks. Picks the cheapest tier that carries the change first (none, lean ritual, full), installs missing tools itself, sends in-flow decisions to TypeSafe Jev, caps design review at three rounds, and never hands the user a command to run. EGM stays off unless asked for. Triggers on: ritual full, full ritual, full flow, plan this feature with every tool, ritual with semantic isa, ritual with egm."
---

# ritual-full

The same flow as [ritual](../ritual/SKILL.md): same steps, same files, same `ritual-*` lines on
the epic, same gates. Read that skill first; this one states only what changes. It also uses
[semantic-isa](../semantic-isa/SKILL.md). EGM is off here too: it runs only when the user asks
for EGM ([ritual's opt-in table](../ritual/SKILL.md#opt-in-egm)). Install and a worked example:
[ritual, zero to hero](https://github.com/decebal/decebal-claude-skills/blob/main/docs/ritual-zero-to-hero.md).

## You run everything

Never give the user a command, a slash command or an install step to run, and never wait for a
file the user is meant to produce. Every command here and in [ritual](../ritual/SKILL.md) is
yours: installs, `cn init`, EGM skills (read the skill file and follow it), `cn approve`,
`cos ritual approve --yes`. A step that needs the user needs a decision from them, asked in
conversation, never an action.

## Decisions: TypeSafe first, the user last

A question the flow would put to the user goes to TypeSafe Jev first, except the three in the
last bullet below. That covers Think's clarifying questions and its choice of approach, Plan's
gate and test-layer questions ([plan.md](../ritual/references/plan.md)), an EGM checkpoint,
whether a review finding is a design defect or Plan detail, and the tier below.

```bash
ritual-feedback decide --question "<one complete question>" \
  --option "<name>=<what choosing it means>" --option "<name>=<what choosing it means>" \
  --state-file <the brief, design section or finding, as text or JSON>
```

It prints `decide: <choice> <policy> confidence=<c> · runner-up <name> p=<p>`:

| Policy | Confidence | You |
|---|---|---|
| `act` | ≥ 0.80 | take the choice |
| `act-and-report` | 0.50 to below 0.80 | take the choice; name it and the runner-up in the step report |
| `ask` | < 0.50 | ask the user that one question in conversation, with the options |

- The state carries the facts the choice depends on, with evidence. It leaves the machine: no
  secrets, no customer data, no client names.
- A non-zero exit (no key, timeout, HTTP error) means decide on your own judgment and report
  `decide: unavailable (<error>)`.
- Three decisions never go to Jev and stay with the user, asked in conversation: design approval
  ([below](#spec-two-reviewers-one-approval)), what to do after the third review round
  ([below](#spec-review-rounds-converge-in-three)), and any irreversible action outside the repo
  (a deploy, a publish, a payment, a delete).

## Tier (first thing, before preflight)

The full toolset costs a preflight, two-reviewer rounds, a Semantic ISA section and hires. Run
the cheapest tier that carries the change:

| Tier | Runs | When |
|---|---|---|
| `none` | no flow: make the change, run the tests, ship it | mechanical: a rename, a typo, a version bump, a doc or comment edit, or a one-line change; no new behaviour, no new test needed |
| `lean` | [ritual](../ritual/SKILL.md) as written; this skill's other sections do not apply beyond this one, [You run everything](#you-run-everything) and [Decisions](#decisions-typesafe-first-the-user-last) | new behaviour in one repo, no new or changed side effect, reversible, a design that fits one page |
| `full` | everything in this skill | a new or changed side effect (network call, API, message, deploy, data write), a production system, more than one repo, auth, money or data deletion, anything that cannot be undone, or an unknown rated water |

1. Read the facts first: the files and modules the change touches, its side effects, the repos,
   whether it can be undone. One `full` fact settles it without a call.
2. Otherwise ask, with the three rows above as options and the facts as JSON:
   `ritual-feedback decide --question "Which ritual tier does this change need?" --option
   "none=<row>" --option "lean=<row>" --option "full=<row>" --state '<facts as JSON>'`. Follow
   its policy. When it is unavailable, run `full`.
3. Only the user's own words override: "full ritual, no downgrade" runs `full`, "lean" runs
   `lean`. Being invoked as `ritual-full` is not that instruction.
4. Report `ritual-tier: <tier> because <facts> (<decide line>)`; for `lean` and `full`, append
   the same line to the epic once it exists. Move up a tier the moment a step finds a side effect
   the tier missed; never move down mid-flow.
5. A bug fix with a clear reproduction skips Think and Spec at every tier, as in ritual. At
   `full` its Review still runs the sink-to-VERIFY check on any sink the fix touches.

## Preflight (tier full, before Think)

Run `ritual-feedback preflight --full --json`. It caches each spawning check (`cn --help`,
`allstack doctor`, `graphify --help`, `cos ritual check --help`) in `~/.ritual/preflight.json`
against the binary's path, size and mtime; only `ok` results are reused, file checks run live.
Pass `--recheck` when the user asks for a fresh check. When a tool fails in real use, run
`ritual-feedback preflight --full --stale <tool>` (`cn`, `allstack`, `graphify` or `cos`): it
rechecks that tool.

The binary's rows `cn`, `allstack`, `graphify`, `feedback` and `cos` apply as reported. Its
`semantic-isa`, `claude-guard` and `skills` rows read `~/.claude/` paths: ignore them on Codex and
check the `skills` and `codex-guard` rows below yourself (file reads, no process). Because those
rows can fail it, decide from the JSON rows, not the exit code. For a `missing` row, run its
install command yourself and check again; do not ask first, and never skip one silently. "This
repo" is a decebal-codex-skills checkout: use the current one, else clone it to
`~/Projects/decebal-codex-skills`; clone other repos the table names beside it under
`~/Projects/`. Build a missing `graphify-out/` yourself. Stop only when an install fails, with the
command and its error. Where the binary's install hint differs from the table, use the table.

| Tool | Row | Check | Install when missing |
|---|---|---|---|
| Chronis `cn` | hard | `cn --help` exits 0; `.chronis/` in the main checkout | `cargo install chronis`, then `cn init` in the repo |
| allstack | hard | `allstack doctor` exits 0 and prints `"executables_ready": true`; `setup_ready: false` is reported, not blocking (the ritual never runs `allstack init`) | clone `all-source-os/allstack`, run `cargo install --path . --locked` in it |
| skills | hard | `ritual`, `semantic-isa`, `pr-proof`, `systematic-debugging`, `forge-prove-it` under `~/.agents/skills/` or the repo's `.agents/skills/` | `cp -R skills/<name> ~/.agents/skills/<name>` from this repo |
| graphify | hard | `graphify --help` exits 0; `graphify-out/` in the repo, else build it | `uv tool install graphifyy` then `graphify codex install`; the same reinstall fixes `graphify --help` exiting 126 with `bad interpreter` |
| codex-guard | hard | `command -v codex-guard` finds it (it has no `--help`; with no subcommand it prints usage and exits 2); the trusted Codex hooks config registers `codex-guard infra-guard`, `bash-hygiene` and `pr-guard` | `cargo install --path gates/rust/codex-guard` from this repo, then register the hooks as `configs/hooks.json` shows and trust them |
| ritual-feedback | hard | the preflight runs at all (it is this binary) | `cargo install --locked --git https://github.com/decebal/decebal-claude-skills ritual-feedback` |
| cos | soft | `cos ritual check --help` exits 0; when the install fails, hires become `spawn_agent` agents in fresh contexts and approval takes the no-`cos` path in [spec.md](../ritual/references/spec.md#approve) | clone `decebal/chief-of-staff-v2`, run `task setup` in it |
| TypeSafe | soft | `$TYPESAFE_API_KEY` is set or `~/.config/typesafe/key` is non-empty; never print the key | none you can run: a key is a credential. Report it missing once; decisions fall back to your own judgment |

Report: `preflight: cn ok · allstack ok · skills ok · graphify ok · codex-guard ok · feedback ok · cos ok · typesafe ok · 4 cached, 0 checked`,
with `installed` where you installed it and `failed (<command>: <error>)` where the install
failed; the counts are the JSON's `cached` and `checked`.

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
name the other in the design's review history.

The approval is the user's, asked in conversation: name the design, its digest and both
verdicts, and ask whether to approve. On their yes, run `cos ritual approve … --yes` yourself and
quote their instruction in your report ([spec.md](../ritual/references/spec.md#approve)). Agents
never approve on their own initiative, and never on a Jev answer.

## Spec: review rounds converge in three

Past flows ran 11 and 17 review rounds; from about round 5 each fresh reviewer closed the last
round's findings and found new `should` items about test placement, which are Plan detail. So:

1. **Round 1:** both reviewers review the whole design.
2. **Rounds 2 and 3:** each fresh reviewer gets the design and only its own side's previous
   review file (`-a` or `-b`), never the other side's, confirms each earlier finding is closed,
   and raises new findings at `blocking` only. A `should` or `nit` it would have raised goes
   under a `## Carried to Plan` heading after its numbered findings, and the Plan turns each into
   a task. A review is `clean` when nothing blocking is open.
3. **After round 3** with a blocking finding open, stop reviewing. Ask the user in conversation,
   with the open findings listed, whether to approve with them carried to Plan as beads, revise
   once more, or rescope. No fourth round without their answer.

Whether a finding is a design defect or Plan detail is a decision: send it to
`ritual-feedback decide` with the finding and its evidence as state.

When the design touches a live system or more than one repo, skip the `cos` hire and start the
fresh `spawn_agent` reviewer directly: `cos` refuses those hires with "can't reach a live system
from its sandbox".

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
