# G5 — the interview

**At most seven questions, and none the census already answered.**

An interview that re-asks what the tree said trains the author to answer carelessly, and a
carelessly-answered question is worse than an unanswered one: it ships as a fact, with no marker
saying to doubt it. So the gate on every question below is the same — *did G1–G4 fail to settle
this?* If they settled it, do not ask; state what you found and move on.

**Every question carries a default, and the default is the honest one.** Silence, a shrug, or
"not sure" is a legitimate answer: the slot ships as `UNVERIFIED:` with the command that would
settle it. It then prints in every review the generated skill produces, which is exactly the
right amount of pressure — visible, cheap to fix, and never mistaken for a fact.

## The seven, ranked by how often a tree cannot answer them

### Q1 · The plane split

> Some behaviour here is shipped in a binary and some is published as data — configuration, a
> rules pack, prompts, a per-tenant manifest, feature flags, a migration seed. When a
> customer-facing output is wrong, which of those is the **default** repair site?

**Why a tree cannot answer it.** Both sites compile. The one that is correct is a policy, and the
wrong choice is invisible in review: a patch in the binary repairs one field while the data still
does not state the contract, so every sibling field fails identically and each needs its own
shipped patch.

Feeds L1. Skip if G2 found exactly one deployable and no published-data surface.

### Q2 · Canonical seams, where two candidates tie

> `{{a}}` and `{{b}}` both look like the owner of `{{responsibility}}`. Which should new code
> use, and is the other on its way out?

Ask **only** for the ties G2 could not break by import frequency. Cap this at three, and ask them
as one question with three lines rather than three questions.

Feeds L2.

### Q3 · The declared layer boundary

> Is there a layer or import direction that is supposed to hold here — and is it asserted
> anywhere, or is it convention?

**Never infer a boundary from directory names.** A generated layer rule nobody agreed to is a
reviewer arguing with its own author, and it is the fastest way to get the whole skill
disbelieved. If the answer is "convention", record it as convention: L1 then reports a crossing
as `sand` with the convention quoted, not as a defect.

Feeds L1. Skip if G2 found an architecture test, an import lint, or a layer gate — that **is** the
declaration.

### Q4 · The surfaces with no runbook

> Which parts of this product can only be verified by a person driving it, and which of those
> have **no** written click-through?

This is the question that fills block two, the deliverable. A surface with no runbook is not a
gap in the census — it is the most useful thing the reviewer can tell someone, provided it says
so plainly instead of inventing steps.

Where the answer is "no runbook exists", the generated hand-check emits the substitute question
rather than fabricated steps:

```
then   no runbook exists — describe the run you did
ask    "Which <surface> did you exercise end to end, and what did you observe at <the step>?"
```

Feeds Phase 4. Ask always.

### Q5 · What "done" means here

> Is there a checklist every feature owes — accessibility, privacy, telemetry, an ADR, a
> migration note? Where is it?

A reviewer that drops its own repo's feature checklist is absurd, and the checklist is almost
never discoverable by grep. If one exists, every `ships-behaviour` claim owes it.

Feeds Phase 4. Skip if G4 already found it in the instruction files.

### Q6 · The state stores

> When a reviewer needs to check what actually happened at runtime, which store holds the answer
> — and is there more than one that could be read by mistake?

**The wrong store returns an empty result byte-identical to "no such data".** Where more than one
exists, the generated skill requires every runtime claim to **name the store it read**, and says
which one answers which kind of question.

Feeds the hand-check item's `read <store or log, with the exact query>` field. Skip if there is
one obvious store, or none.

### Q7 · The sanctioned moves

> Which refactors are this repo's *prescribed* way of doing something — splitting a file to get
> under a size cap, adding a sibling module, marking a network test skipped?

Each becomes a **carve-out** in discipline rule 10 rather than a finding: a sanctioned shape is a
finding only under a named extra condition, and the condition must be stated. Without this the
generated reviewer reports the repo's own recommended move as a defect on its first run, and
nobody runs it twice.

Feeds discipline rule 10. Skip if G4 found them in the instruction files.

## Questions never to ask

| Do not ask | Because |
|---|---|
| "What are your gates?" | G1 read them. Asking says the census did not run. |
| "What's your architecture?" | Unanswerable in the abstract; Q3 asks the decidable half. |
| "What should the reviewer look for?" | It produces a wish list, not facts. Incidents come from G4, which has evidence. |
| "How long does X take?" | No duration appears anywhere in this skill or its output. |
| Anything with a yes/no default that does not change what gets written | A question whose answer changes nothing is noise, and it spends the budget of one that would. |

## Recording the answers

Each answer is written into the generated skill **with its provenance**, in the same shape a
census fact gets:

```
plane split: published data is the default repair site   [author, forge-prove-it interview]
layer direction: convention only, not asserted            [author, forge-prove-it interview]
```

The tag matters. A reader who disagrees with a fact needs to know whether to re-run the census or
re-ask a person, and those are different actions.
