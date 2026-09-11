---
name: forge-prove-it
description: "Read this repository's real enforcement surface — which gates exist, which fire, which look at nothing, and which can actually block a merge — then write a prove-it feature reviewer calibrated to it. Use when a repo needs its own adversarial feature reviewer, when asked to generate, forge, bootstrap or port prove-it into a codebase, when a team keeps shipping features that pass every gate and still do not work, or when an existing prove-it has drifted from the tree it reviews. Emits a skill whose every claim is either a mechanism true in any repo or a fact this run observed, with the observing command recorded beside it."
---

# forge-prove-it

Writes a **prove-it** into this repository: an adversarial reviewer that asks whether a feature
is real — right layer, reusing existing seams, weakening no guard, no cosmetic work, wired end to
end and past the happy path — and ends with the hand-checks a human still owes.

prove-it only works when it is **calibrated**. Its power is not the six lenses; it is knowing
that *this* gate's trigger is wider than its scan root, that *that* suite runs in no job at all,
that this repo's last four incidents were all second-item failures. Those facts are not
transferable, and a reviewer that guesses them is worse than no reviewer: it is a confident,
plausible, wrong one, and plausible-and-wrong is the failure mode that survives review.

So this skill is a **census**, not a template.

## The one rule

> Every line written into the generated skill is either **(a)** a mechanism that holds in any
> repository, or **(b)** a fact this run observed, carrying the command that observed it.

There is no third category. A slot the census could not fill is emitted as an
`UNVERIFIED:` marker naming the question and the command that would settle it — never filled
with a plausible default. A generated reviewer citing a gate that does not exist teaches its
readers to disbelieve it, and they are right to.

Three corollaries, each of which has its own failure mode:

**Absence of a gate is a finding, not a gap in the census.** "Nothing checks this" is prove-it's
most valuable output. Record it with the search that established it.

**A row with no local equivalent is deleted, not translated.** If this repo has no per-tenant
data, no customer-specific code, no second deployable — the probe for it matches nothing forever
and teaches the reader that the lens is empty ritual. An empty ritual reads exactly like a
passing check, which is the defect this whole design exists to surface. Dropping is mandatory.

**Cite symbols, never line numbers.** A line number is a time bomb: in one repo, two rule docs
disagree about the same code today and one names a test symbol that was renamed. Emit the
greppable name plus the standing instruction to re-grep before a finding rests on it.

## 1. Arguments

```
$forge-prove-it [--out DIR] [--host claude|codex|both] [--name SKILL-NAME]
                [--interview | --no-interview] [--update] [--dry-run]
```

| Input | Effect |
|---|---|
| *(nothing)* | census this repo, write to the host's skill directory, interview on what the census could not settle |
| `--out <dir>` | write there instead |
| `--host` | `claude` → `.claude/skills/<name>/`; `codex` → `.agents/skills/<name>/` for repo scope, `~/.codex/skills/<name>/` for user scope; `both` → both, same content, host-specific §Phases (see `references/host-differences.md`). Default: whichever host directory already exists; if neither, ask. **Confirm the scan path against the host's own docs before writing** — a skill written where the host does not look is invisible, and it fails silently. |
| `--name` | default `prove-it` |
| `--no-interview` | census only, and **nothing blocks**. Every unsettled slot ships as `UNVERIFIED:`. Legitimate — a marked gap is honest; a guessed answer is not. With neither host directory present, default to `claude` and **print which was chosen** rather than asking, since asking is what this flag turns off. |
| `--update` | an existing generated skill is re-censused. Facts are **replaced**, hand-written prose between `<!-- keep -->` fences is preserved, and every fact that changed is printed as a diff. |
| `--dry-run` | print the census and the file list; write nothing |

**Writes files. Nothing else.** Creates no branch, no commit, no push; runs no build, no test
suite, no compiler, no package manager. The census is reads and greps. A skill that detects
unwired gates must not become one, so it also ships no gate script of its own.

## 2. The phases

| Phase | What | Judged? | Agents |
|---|---|---|---|
| G0 | Frame — host, stacks, existing skill, repo identity | mechanical | 0 |
| G1 | **Enforcement census** — hooks, CI, blocking status, the orphan check | mechanical | 0–1 |
| G2 | **Structure census** — workspaces, layers, planes, canonical seams | mechanical + judged | 1–3 |
| G3 | **Guard census** — ratchets, allowlists, ignore files, invocation flags | mechanical | 0 |
| G4 | **Incident mining** — git history, docs, the repo's own instruction files | judged | 1–2 |
| G5 | **Interview** — only what G1–G4 could not settle. Capped. | — | 0 |
| G6 | Emit | mechanical | 0 |
| G7 | **Self-audit of the emitted skill** | mechanical | 0 |

Parallelise G1–G4; they share no state. G5 needs all four.

### G0 · Frame

Establish, and print:

- the host — which of `.claude/skills`, `.agents/skills`, `~/.codex/skills`, `AGENTS.md`,
  `CLAUDE.md` exist
- the stacks, from manifests actually present, never from file extensions alone
- whether a generated skill is already here (`--update` path)
- the trunk's name — read it, do not assume `main`. Every delta probe in the emitted skill is
  measured against a base ref, so an unresolvable trunk poisons the whole run; make it mark itself
  instead. In order, stopping at the first that answers:

  ```
  git symbolic-ref --short refs/remotes/origin/HEAD
  git rev-parse --abbrev-ref origin/HEAD
  git branch -r                      # look for the 'HEAD ->' line
  git remote show origin             # NETWORK CALL — can hang; last resort only
  ```

  None of them answers → `UNVERIFIED: trunk not determinable offline ·
  would settle: git remote set-head origin -a`

### G1 · Enforcement census — the load-bearing phase

This produces the generated skill's **coverage ledger**, and the ledger is what stops prove-it
from re-deriving what a gate already owns. Get it wrong in the loose direction and the reviewer
stays silent about real holes; get it wrong in the strict direction and it re-reports what CI
already said.

Three questions per check, and they are genuinely different:

1. **Does it run?** — its trigger.
2. **Does it look at the changed path once running?** — its scan root.
3. **Can it stop a merge?** — its authority.

A check whose trigger is wider than its scan root **fires and reports success having looked at
nothing** — and it is invisible to everyone downstream, because a green check and a green check
look identical.

Commands per ecosystem — hooks, CI, blocking authority, the trigger/scan-root split, and the
orphan sweep: `references/enforcement-census.md`.

**The orphan sweep is the highest-yield probe in this whole skill, and it is mechanical —
but it is transitive reachability, not a grep.** Enumerate every declared check, then close the
graph: a target reached only through an aggregator is not an orphan, a target resolved by its
implementation path rather than its alias is not an orphan, and a target a framework invokes by
name (a lifecycle script, a hook id) is not an orphan either. A check genuinely reachable from
nothing runs nowhere — and one that runs nowhere is indistinguishable from one that passes.

**Authority is not assumed.** Where the host is GitHub, read it:

```
gh api "repos/OWNER/REPO/branches/TRUNK/protection"
gh api "repos/OWNER/REPO/rulesets"
```

`404` means no protection is configured. `403` means the plan does not offer it — on a private
repo under a free org, protected branches are unavailable **by plan, not by misconfiguration**,
so every check is advisory and a human can merge past a red run. Either way the generated skill
carries a standing footer saying what CI can and cannot do here. When it cannot be determined,
the footer says `UNKNOWN` — never "CI covers it".

### G2 · Structure census

Feeds the generated L1 (placement) and L2 (reuse).

- **Units** — workspace members from the manifests that exist (`Cargo.toml` `[workspace] members`,
  `package.json` `workspaces`, `pnpm-workspace.yaml`, `go.work`, `pyproject.toml`, `composer.json`).
- **Planes** — where each unit *runs*: shipped binary, server, build-time tool, published data.
  The data-versus-binary split is the one L1 cannot compute and must ask about.
- **Layers** — taken from a declared boundary (an architecture test, a layer gate, an import lint,
  an ADR) if one exists. **If none exists, say so and do not invent one from directory names.**
  A generated layer rule nobody agreed to is a reviewer arguing with its own author.
- **Canonical seams** — the shipped helpers a new file should have reused. Found by import
  frequency, then confirmed by the author; never guessed from a name.

Per-language discovery commands and header conventions: `references/structure-census.md` §4.
The bounded 8-sibling header-read algorithm that makes L2 terminate is part of the emitted
skill, and lives with it: `references/emitted-skill-template.md` §L2.

### G3 · Guard census

Every ceiling, allowlist, ignore file and suppression this repo has, so the generated L3 can tell
a tightening from a loosening. **Direction lives in the command, never in the brief** — a grep
matching `[+-]` matches both directions identically and is a coin flip wearing a check. Pair the
tuples by key and compare the values, or do not ship the detector.

Record for each: the file, the key, its value today, and whether the list is **closed and empty**
— a first entry in an empty allowlist is the strongest single signal any prove-it can have.

`references/structure-census.md`, §Guards.

### G4 · Incident mining

The rows that make a prove-it worth reading. Four sources, cheapest first:

| Source | Command shape |
|---|---|
| The repo's own instruction files | `CLAUDE.md`, `AGENTS.md`, `.cursorrules`, `.github/copilot-instructions.md`, `.claude/rules/`, `CONTRIBUTING.md` |
| Revert and hotfix history | `git log --oneline --grep` over revert/hotfix/regression/postmortem/incident |
| Decision and report directories | `docs/decisions/`, `docs/adr/`, `docs/reports/`, `docs/postmortems/`, `RFC*/` |
| The tracker | `gh issue list --label bug --state closed`, `gh pr list --search "revert"` |

**A prohibition in an instruction file is an incident with its story removed.** "Never use
`navigator.clipboard` directly" means someone shipped it and it silently no-opped. Convert each
prohibition into a probe: the banned construct is the grep, the sanctioned one is the kill, and
the prohibition's own sentence is the finding text. Where the file records the cost, keep the
cost — a bare prohibition gets rationalised away by the next person under deadline; one with a
price attached does not.

Method and the conversion table: `references/incident-mining.md`.

### G5 · Interview — bounded, and only on what is genuinely unsettled

**Ask at most seven questions, and ask none the census answered.** Interviews that re-ask what
the tree already said train the author to answer carelessly.

The questions that are almost always genuinely unanswerable from a tree — the plane split, what
"done" means here, which surface has no runbook, which of two seams is canonical — with wording
and defaults: `references/interview.md`.

Every unanswered question still ships, as an `UNVERIFIED:` line in the generated skill, with the
command that would settle it. A skipped interview degrades the output honestly; a guessed answer
does not.

### G6 · Emit

Write the calibrated skill. Its shape — the six lenses, the phase pipeline, the claim-shape
table, the discipline rules, the output block order — is portable and reproduced verbatim from
`references/emitted-skill-template.md`. What the census supplies is every **fact** inside it.

Written **into the target repo** — these are the generated skill's own files, not this one's:

| Emitted file | Filled from |
|---|---|
| `<out>/SKILL.md` | G0 identity, G1 ledger summary, the phases, the lens spawn-gate table |
| `<out>/references/coverage-ledger.md` | G1 in full — the four buckets, the trigger/scan-root mismatches, the authority footer |
| `<out>/references/placement-map.md` | G2 units, planes, layers |
| `<out>/references/patterns-index.md` | G2 seams + the bounded sibling algorithm |
| `<out>/references/guard-ratchets.md` | G3 |
| `<out>/references/cosmetic-probes.md` | the universal probes, plus G4-derived ones |
| `<out>/references/wiring-and-second-item.md` | G2 chains + G4 incidents |
| `<out>/references/manual-verification-routes.md` | G1 buckets 3 and 4 + G5 runbook answers |
| `<out>/references/discipline.md` | portable, verbatim |

**Emit only the files the census can fill.** A repo with no ratchets gets no `guard-ratchets.md`;
the count above is the maximum, not a quota. An emitted file with nothing in it is the empty
ritual this skill exists to avoid.

**Lens count is not fixed at six.** A lens whose surface this repo does not have is emitted as a
one-line stub saying so and why. Six lenses on a repo with one deployable is five agents burning
context to find nothing.

### G7 · Self-audit of the emitted skill

The generator's own output is testimony, and until this phase nothing read it. Five mechanical
checks over the written files, zero agents:

| # | Check | On failure |
|---|---|---|
| A1 | Every path cited resolves | rewrite to the real path, or mark `UNVERIFIED:` |
| A2 | Every command cited is runnable — and for a task target, **the target itself exists**, not just its runner | mark `UNVERIFIED:` with what is missing |
| A3 | Every quantifier carries the command that established it — the trigger set and its killers are defined once, in `references/self-audit.md` | **rewrite to the bounded observation, and print the rewrite** |
| A4 | Every emitted probe pattern has matched a line somewhere in this tree | fix the boundary, or label it `pattern unvalidated` |
| A5 | No `UNVERIFIED:` was silently dropped | restore it |

A3 is the one that matters. The claim is usually true and always cheaper to bound than to prove:
"no test exercises that ordering" becomes "`git grep -n seal_user_token` finds one construction,
unconditional, at `…:67`". **The rewrite is printed.** A silent rewrite is a masking guard, and
this skill does not ship one.

Print the audit as a footer, listing the bounded rewrites beneath it:

```
SELF-AUDIT · {{n}} paths resolved · {{n}} rewritten · {{n}} quantifiers bounded · {{n}} UNVERIFIED
```

The five checks in full, with their killers and the footer shape: `references/self-audit.md`.

## 3. Output

**The shape, in slots.** Every value below is filled by the census. No example values are given
here on purpose: a filled sample is a set of facts about somebody else's repository, and a reader
— or a model — that pattern-matches on it will emit those facts about this one. That is the
failure this whole skill exists to prevent, and a worked example is the most effective way to
cause it.

```
forge-prove-it · {{repo}} · {{trunk}}@{{sha}} · host: codex · {{n}} units · {{n}} stacks

ENFORCEMENT
  hooks            {{hook_file}} ({{n}} gates) · {{hook_file}} ({{n}})
  CI               {{workflow_files}}
  can block?       {{YES, naming the required checks | NO, with the command and its response
                    | UNKNOWN, with the command that would settle it}}
  orphans          {{n}} — {{ids}}   [declared, reachable from nothing]
  blind            {{n}} — {{id}}: triggers on {{trigger}}, scans {{scan_root}}

CENSUS
  units · planes · layers · seams · chains · guards · incidents
  — each row carrying the command that established it

UNVERIFIED — {{n}}
  · {{the question}}        would settle: {{the command, or "ask the author"}}

WROTE
  {{out}}/SKILL.md  +  {{n}} references/   ({{n}} skipped: {{why}})

SELF-AUDIT · {{n}} paths resolved · {{n}} rewritten · {{n}} quantifiers bounded · {{n}} UNVERIFIED

NEXT
  1. answer the UNVERIFIED lines, or leave them — they print in every review
  2. run it on a merged PR you already know the truth about; that is the calibration test
```

`can block?` has three states and they are not interchangeable — a `NO` and an `UNKNOWN` lead a
reader to opposite conclusions about what the reviewer is for. Print the command and its response
beside whichever one you emit.

## 4. Verifying the generated skill

**Run it on a PR whose outcome you already know.** A reviewer is calibrated when it finds the
thing that actually broke and says nothing about the things that did not. Pick a PR that shipped
a defect; if the generated prove-it misses it, the census missed the fact that would have caught
it, and that is the row to add.

Re-run `--update` when the enforcement surface moves — a gate added, a check demoted to advisory,
a scan root narrowed. **A stale ledger is the failure this skill exists to prevent**; a reviewer
carrying one delegates to a gate that no longer looks.

## 5. What it does not do

| Not done | Why |
|---|---|
| Fill a slot with a plausible default | the only rule this skill has |
| Invent a layer boundary nobody declared | a reviewer arguing with its author |
| Assume `main`, or that CI can block | both are read |
| Run a build, a test suite, or a package manager | the census is reads and greps |
| Branch, commit, or push | it writes files |
| Ship a gate script | rule 13 of the emitted discipline applies to the generator too |
| Emit a numeric confidence or any duration | `rock` / `sand` / `water`; a threshold makes 79 and 81 categorically different for no measurable reason |
| Store a count about the tree | a stale inventory is what it is built to catch |

Two rules in this collection carry the reasoning behind the census: `testing-gates` (what actually
enforces anything) and `evidence-discipline` (check the destination before trusting an absence).
They are named rather than linked — a skill directory that links outside itself stops resolving
the moment it is copied into a skills directory on its own.
