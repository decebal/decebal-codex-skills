# The emitted skill — portable shape, census-filled facts

What G6 writes. Everything in this file is **portable**: it holds in any repository and is
reproduced as-is. Every `{{slot}}` is filled from the census, and an unfilled slot ships as
`UNVERIFIED: <question> · would settle: <command>` — never as a plausible default.

Read this alongside `enforcement-census.md` (fills the ledger), `structure-census.md` (fills L1,
L2, L3) and `incident-mining.md` (fills L4, L5, L6).

---

## The frame the emitted skill states about itself

> The unit of review is a **claim** ("this ships X"), not a line. The question is always *"is
> there a path where X does not happen?"* — never *"is this code wrong?"*.

Two properties keep it above the noise floor of a generic opinion generator, and both must
survive into the emitted copy:

1. **It only reviews what no gate covers.** The un-gated set is computed, not guessed.
2. **A finding must be reproduced, and a kill must be evidenced.** Neither side gets to assert.

Drop either and what remains is a linter with worse recall.

## The phase pipeline — portable, verbatim

| Phase | Mechanical / judged | Agents |
|---|---|---|
| 0 — Frame: range, liveness, claims, hunk coverage | mechanical + one echo | 0 |
| 1 — Ledger + owned probes | fully mechanical | 0 |
| 2 — Refuters, gated by diff surface | judged | 1 per triggered lens |
| 3 — Counter-refutation, scaled by finding count | judged, evidence-bound | 0–2 |
| 4 — Manual routing | mechanical table join | 0 |
| 5 — Emit | mechanical | 0 |
| 5.5 — Self-audit of the assembled emission | fully mechanical | 0 |

**Phase 1 runs every probe at HEAD *and* at the base ref in the same session and reports only the
delta.** No counts are stored anywhere in the emitted skill. A stale inventory is the defect this
whole approach exists to catch, and shipping one would be that defect wearing a review badge.

**Phase 3 scales with findings**: zero findings spawns zero counter-refuters; one or two spawns
one; three or more spawns two. Join the verdicts to the findings **in code** and hand the report
step only the joined result — re-sending both raw payloads so a model can do a deterministic
match is the most expensive no-op in the pipeline.

## Claim shapes — portable

| Shape | Lenses | The thing to refute |
|---|---|---|
| `ships-behaviour` | all triggered | the behaviour never happens on some path |
| `fixes-defect` | guard, wiring, second-item | the invariant is unasserted, or a sibling call site still has the bug |
| `adds-guard` | guard, cosmetic | the guard is unwired, or its pattern matches nothing |
| `refactor-no-behaviour-change` | reuse, guard | the **absence** of behaviour change |
| `NOT A CLAIM` | none | "improves reliability" asserts nothing; say so rather than hunting it |

Also emit a **hunk-coverage line**: every changed hunk maps to at least one claim, or is listed
`unclaimed`. The drive-by refactor nobody asked for costs one line to surface and is a standard
signature of machine-written changes.

## The coverage ledger — four buckets, portable

Only the first silences the panel.

| Bucket | Meaning | Effect |
|---|---|---|
| **FIRES, covers it** | selected, and its scan root covers the changed paths | **DELEGATED** — the panel is barred from re-deriving anything in its remit |
| **FIRES, partially blind** | selected, but its scan root is narrower than its trigger | **REVIEWED.** A firing gate must never suppress a hole inside its own remit |
| **EXISTS, did not fire** | declared, remit touched, trigger missed it | hand-check: one command, exit-0 pass |
| **Nowhere, ever** | orphaned, or ungated | the panel's own beat |

`{{ledger}}` — filled by G1, one row per check, with its trigger, its scan root, and its bucket.

`{{authority_footer}}` — the standing footer, printed on every review. One of:

- *"No check on this repository can be required — `{{command}}` returned `{{result}}`. A red run
  is visible and a human can merge past it."*
- *"`{{n}}` checks are required on `{{trunk}}`: `{{names}}`. Anything not in that list is advisory."*
- *"UNVERIFIED: blocking authority could not be determined. `{{command}}` returned `{{result}}`."*

Never *"CI covers it"*. That sentence is the thing the ledger exists to replace.

## The lenses — portable questions, census-filled probes

Emit a lens only if this repo has its surface. A lens with nothing to attack returns nothing
while costing a full context; a lens stubbed out with a reason is honest and free.

| Lens | Attacks | Emit when | Fill from |
|---|---|---|---|
| **L1 placement** | wrong app / unit / layer / plane | more than one unit, or a declared plane split | G2 |
| **L2 reuse** | an abstraction that reinvents a shipped seam | any repo (the algorithm is bounded and self-contained) | G2 |
| **L3 guard-weakening** | a ceiling raised, a gate that lost a flag, a path moved out from under a named-path list | at least one **ceiling, allowlist, ignore entry or invocation-flag surface** exists — a bare CI job with nothing configurable is not enough | G1 + G3 |
| **L4 cosmetic** | dead code, a test that cannot fail, an unwired gate, doc-only completion | **always** — cheapest lens, applies to every diff | portable + G4 |
| **L5 wiring** | the missing hop, in **both** directions | a multi-step registration chain exists | G2 + G4 |
| **L6 second item** | the 2nd iteration, the failing element, the concurrent run, the abort | **always**, unless the repo has no loop, queue or state machine anywhere | portable + G4 |

**The spawn gate errs toward spawning.** A skipped lens costs nothing and finds nothing; a lens
that should have run costs a defect.

**L6's trigger is deliberately the widest.** Not just loops, queues and batches — *any state
decision*: a function returning one of several states, a status enum, an early-return guard, an
error arm. A diff adding `if (input.paired) return "loading"` contains no loop and no queue, and
L6 is still the only lens that asks the question that matters there — *which states reach that
branch and never leave it?* Narrow the trigger to the obvious tokens and the highest-yield lens
skips the diffs it was written for.

**Print the skipped lenses and why.** A silently skipped lens is indistinguishable from one that
found nothing, which is the failure the whole design is built against.

### L6, the four questions — portable, verbatim

For every loop, batch, queue, retry, park, lease, slot or state transition the diff touches:

1. **What happens to the second element?** Not the first. The second, and the last. Is anything
   about element *n* derived from a bound element 1 happened to satisfy — a length cap, an index,
   a preallocated buffer, an id built from a growing prefix?
2. **What happens when one element fails?** Does the failure abort the batch or skip the element?
   Is the error attributed to the element that produced it, or to whatever the loop variable
   happens to hold? Does a partial failure leave the batch half-committed?
3. **What happens on the exit paths — every one of them?** Enumerate the states that release a
   resource and the states that hold it, and confirm the diff's new state is in exactly one list.
   A new terminal state, a new park, a new review gate and a new abort are each a door.
4. **What happens when it runs twice, or concurrently?** Is the id deterministic, and is that
   intended idempotency or an accidental swallow? Does a second concurrent run see the first's
   partial state? Is "already exists" handled as success or retried?

`{{l6_incidents}}` — G4 attaches this repo's own incidents to whichever question they answer. An
incident under a question is what makes it get asked; a bare question gets skimmed.

### L6, second half — a fix that names no invariant

On a `fixes-defect` claim, two questions beyond "does the diff contradict the regression commit":

**(a) Is the invariant asserted anywhere it can be re-violated?** A fix that removes only the
symptom gets re-broken through the next door. Ask the author to state the invariant in one
sentence; if they can, ask where it is asserted.

**(b) Do the sibling call sites have the same bug?** This is the highest-value question in the
whole reviewer and it is one grep. Find the fixed construct's siblings — the other fields in the
same struct, the other arms of the same match, the other callers of the same helper, the other
members of the same array — and check whether the diff fixed one or all. If one, say which were
left and let the author decide. That is a finding, not a nit.

### L2 — the bounded sibling algorithm, portable

Without a stopping rule "is there an existing owner for this abstraction" is unanswerable, and
the lens silently degrades into running a fixed list of greps, which is L4's job. So, for each
**new file or new exported symbol**:

1. **Nearest siblings.** List the directory the new file landed in; if the diff added the
   directory, list its parent. Cap at the **8** siblings whose names share the most tokens with
   the new file's.
2. **Read their headers, not their bodies.** `{{header_convention}}` — per-language, from G2.
   That is the budget. Do not read 8 full files.
3. **Decide, and say which.** A sibling's header describes this responsibility → **finding**,
   name it and quote the header line. No sibling owns it and the new file's header says why it
   exists → **not a finding**. No sibling owns it and the new file has **no header** → `sand`,
   routed as "say why this is new" — not a defect.
4. **Stop.** One pass, no widening. No siblings at all → print `no sibling set — new subsystem,
   reviewed by L1 for placement instead`.

State the candidate count so the bound is auditable: `L2: 3 new symbols, 19 siblings read, 1 finding`.

### L4 — the universal probes

These need no repo knowledge and are emitted into every generated skill. Each is a **delta**: run
at HEAD and at base, report only what the diff added.

**Every probe needs an explicit word boundary and a false-positive kill list.** Measured: a naive
`xit\(` sweep returned 15 hits, every one of them the tail of `process.exit(`; the word-bounded
form `(^|[^A-Za-z0-9_.])(xit|fit)\(` returns zero, which is correct. The same class bites `fit(`
inside `benefit(`, `.only(` inside `readonly(`, and an ignore attribute inside a doc comment
quoting it. Without boundaries the generated skill emits a page of noise on its first run and
gets muted — and a muted reviewer is worth less than none.

| Id | Probe | The kill that must be ruled out first |
|---|---|---|
| P1 | A listener/handler with nothing producing its event | the name is built by string interpolation, held in a constant, or produced from another language |
| P2 | An enum variant or case constructed nowhere | a deserializer is an invisible producer — read the derive/decorator line and grep the **wire string**, not the identifier |
| P3 | A test that re-declares its subject | none — a test declaring a function that shares a name with an export of the module it names is testing its own copy |
| P4 | A test green on absent input | an env-var early return, a skip whose reason names an external resource (convention) rather than an unlanded fix (finding), a filter matching nothing, a mock so complete the assertion can only be true |
| P5 | An unwired gate — **state the tier** | it is wired in the blocking tier and the id strings match **exactly** |
| P6 | A gate whose own pattern cannot match | test every regex the diff adds against a real line from a file inside that gate's own roots |
| P7 | An unreachable component or module | a dynamic import, a mock path string, a string-keyed route — and exclude build output directories, or the sweep finds everything reachable |
| P8 | Config theatre — a flag, setting or env var nothing reads | it is read by a consumer outside this repo; an env var referenced only in a guide is documentation, not configuration |
| P9 | Scaffold on the feature's own path | off the claim's path this is ordinary; on it, the claim is refuted |
| P10 | Doc-only completion | pick the doc's single most concrete claim and grep the **implementation**, not the name |

P5's tiers, filled by G1: `{{gate_tiers}}` — at minimum, *wired into the blocking hook* /
*CI-only* / *runs when a human types it* / *nowhere*. Tiers 3 and 4 are the finding.

**The two-consumers-disagree case is worth the extra second.** Compare the id **strings**, not
their meanings: a classifier emitting `SCOPE-api` while the hook tests for
`SCOPE` means the steps never ran once, and both files read as correct.

## Discipline — the false-positive controls, portable

| # | Control |
|---|---|
| 1 | **Delegation exclusion** applies only to a check that both fires *and* covers the path. Never to an unknown trigger, never to a partially-blind gate. |
| 2 | **Counter-refuters get the documented killer for the exact signal** that produced the finding, pre-loaded. |
| 3 | **Reproduce or drop.** A defect carries a command that was run with its output pasted, or a `file:line` quoted this session. An unproven suspicion routes to a human — the correct destination, not a bin. |
| 4 | **A kill needs evidence too.** An unevidenced kill does not kill. Without this symmetry kills are cheaper than findings and the panel under-reports. |
| 5 | **Deltas only, measured twice.** No stored counts. |
| 6 | **Direction lives in the command, not the brief.** A grep matching `[+-]` matches both directions identically; pair the tuples by key and compare, or the detector is a coin flip dressed as a check. |
| 7 | **Distrust prose, trust config.** Re-grep every cited symbol before a finding rests on it. A stale line number in a rule doc is doc drift, **never** a code finding. |
| 8 | **A gate's own warning is a lead, never a finding.** Verify against the tree; name which gate you distrusted. |
| 9 | **rock / sand / water.** Only `rock` is a defect. `sand` is a hand-check carrying a pass criterion. Never a number, never a duration. |
| 10 | **Sanctioned shapes carve out.** `{{sanctioned_moves}}` — the repo's own prescribed moves are findings only under a named extra condition. |
| 11 | **Environment is not a defect.** An unreachable service, a missing fetched asset, a cold build, a fresh checkout. **An unavailable tool is a connection failure, never evidence of absence.** |
| 12 | **A subagent's report is testimony.** Re-read every cited `file:line` at HEAD; a citation that does not say what the finding claims drops it, and **the drop is printed**. Cross-agent agreement gives **no** confidence boost. |
| 13 | **Self-consistency.** Ships no gate script and runs no build. A skill that detects unwired gates must not become one. A probe that earns permanence gets promoted to a real gate and wired in, in the same change. |
| 14 | **The orchestrator's own prose is testimony too.** Phase 5.5 reads it: a quantifier carries the command that establishes it, or is **rewritten to the bounded observation and the rewrite printed**. |

## Output — portable, order is load-bearing

| # | Block | Prints when |
|---|---|---|
| 0 | header | always |
| 1 | `CLAIMS` + `UNCLAIMED HUNKS` | always |
| 2 | `VERIFY BY HAND` | **always** — "0 items, no hand-check owed" is a real outcome |
| 3 | `FINDINGS` | when any survived |
| 4 | `ATTACKS TRIED` | always |
| 5 | `LEDGER` | always |
| 6 | `CONSIDERED AND DROPPED` | always |
| 7 | `NEXT` | when there is a finding worth tracking |
| 8 | `SELF-AUDIT` | always |

`VERIFY BY HAND` is **block two** — never last, never elided, never summarised into "and run the
tests". `ATTACKS TRIED` and `CONSIDERED AND DROPPED` print even when empty, because an
under-reporting panel must be **visible** rather than indistinguishable from a clean diff.

Per claim: `HOLDS` · `REFUTED` · `UNVERIFIABLE HERE` · `NOT A CLAIM`.
Header: `CLAIMS HOLD` · `CLAIM REFUTED` · `UNPROVEN — n outstanding`.

**Hand-checks carry two counts, and only one gates the verdict:**

- **outstanding** — the panel could not settle a claim. This is the signal, and it is what
  `UNPROVEN` counts.
- **routine** — owed because no check watches this path. Never gates the verdict.

Collapse them and `CLAIMS HOLD` becomes unreachable for any diff on an unwatched path, block two
prints forever, and the reader learns to skip the one block that matters.

Every finding carries a **consequence**, judged against *what breaks, and when*:
`BLOCKS-CLAIM` (a claim's own path fails now) · `SILENT-RISK` (works now, the next regression is
invisible) · `WRONG-PLANE` (works, will need re-landing elsewhere).

**Hand-check item shape — all fields resolve, or the item is refused:**

```
N. <claim-id | gate-id> · <what to verify>
   why    <why the repo cannot answer this>
   run    <command, verified to exist this session>
   then   <steps, or "no runbook exists — describe the run you did">
   read   <store or log, with the exact query>
   pass   <an observable criterion, never "looks right">
   caveat <where the runbook itself is unreliable>
   ask    <the substitute question, when `then` has no runbook>
```

**Cap the hand-check list at 6, ranked by claim-criticality**, and print `+N routed, not shown`
when it truncates. A list of twelve is a list of zero, and a silent cap is a silent drop.

## Two things the emitted output never contains

- **A numeric confidence.** A threshold makes 79 and 81 categorically different for no measurable
  reason, and this confidence is a claim about demonstrability, not a probability.
- **Any remark on the size of the diff**, the file count, or how long anything will take.
