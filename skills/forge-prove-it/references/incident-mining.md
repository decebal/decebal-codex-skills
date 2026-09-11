# G4 — incident mining

The rows that make a generated prove-it worth reading. A lens that asks a generic question gets a
generic answer; a lens that asks *"does the second row mint, the way it didn't in March"* gets
looked at.

**A prohibition is an incident with its story removed.** Most repos have already written their
incidents down — as rules, as bans, as a paragraph in a contributing guide — and stripped the
cost out. Mining puts the cost back, because a bare prohibition gets rationalised away by the next
person under deadline and one with a price attached does not.

Four sources, cheapest first. Stop at **8–15 incidents**; past that the marginal row is a
paraphrase of one you have.

**Check first whether this repo has any of them:**

```
git ls-files 'CLAUDE.md' 'AGENTS.md' 'CONTRIBUTING.md' '.cursorrules' '.github/copilot-instructions.md'
git ls-files 'docs/**' 'doc/**' 'rfcs/**' | head
```

A repo with no instruction file, no docs tree, no ADR directory and no tracker yields **nothing**
here, and the generator must say so rather than emit probes with no incident attached:

> `incident mining: no sources found. The emitted lenses carry portable questions only.`

A probe with no cost behind it consumes the reviewer's attention budget and is rationalised away
on first contact — worse than no probe.

---

## Source 1 · The repo's own instruction files — richest per byte

```
git grep -nE '\*\*(NEVER|ALWAYS|Never|Do not|DO NOT)[^A-Za-z]|^- (Never|Always|Do not)[^A-Za-z]' -- <the files found>
```

### The gate check, before anything else

**Check G1's ledger before converting a prohibition into a probe.** A prohibition that a *firing,
covering* gate already enforces is a **bucket-1 delegation**, not a lens probe. Converting it
anyway makes the generated skill re-derive what a gate already owns — the precise thing the
delegation contract forbids, and the fastest way to make a review unreadable.

Measured: one repo's *"never write domain data straight to a file"* is enforced by both an
architecture test and a lint's disallowed-methods list. It belongs in the ledger, not in L2.

### Is it mechanizable?

| The rule | Becomes |
|---|---|
| has a backticked literal, a symbol, an import path, an API name | a **probe**: the banned construct is the grep |
| has no token — *"never render a raw payload"*, *"copy is plain English"*, *"a component's size prop is never a layout tool"* | a **judgement probe plus an interview question**, and the emitted skill says which it is |

**Never fake a regex for the second kind.** A detector invented for an untokenizable rule either
matches nothing — a disabled gate reporting success — or matches everything, which is noise. Both
are worse than saying "this one needs a human", which is a legitimate and useful answer.

### Converting a mechanizable prohibition

| Part of the rule | Becomes |
|---|---|
| the banned construct | the **grep** |
| the sanctioned alternative | the **kill** a counter-refuter gets |
| the rule's own sentence | the **finding text** |
| the recorded cost, if any | the reason the question gets asked at all |
| the exceptions it lists | the carve-outs, verbatim |

Worked: *"Never call the platform clipboard API directly — it silently no-ops inside the app's
webview. Use the wrapper."*

```
id       reinvented-seam/clipboard
grep     the raw API name, across the surfaces that ship inside the webview
kill     the call site is outside those surfaces, or it is the wrapper's own implementation
finding  "raw clipboard call — silently no-ops in the webview; use <wrapper>"
scope    delta only; standing violations are not this diff's problem
```

**Three traps, each of which has produced a wrong row:**

- **Rule docs go stale.** Re-grep every symbol before writing it into a probe. In one repo two
  rule docs disagree about the same code *today*, and one names a test symbol that was renamed. A
  citation that does not resolve is doc drift, never a code finding — and a probe resting on a
  deleted script is a reviewer that cries wolf on its first run. **Emit symbol names, never line
  numbers.**
- **Scope the probe to the delta.** Most prohibitions have standing violations. A probe reporting
  them all on every review is noise by the second run.
- **Keep the exceptions.** A rule listing three sanctioned shapes, converted into a probe that
  knows none of them, reports the repo's own recommended move as a defect.

## Source 2 · Revert and hotfix history

```
git log --oneline -E --grep='revert|hotfix|regression|postmortem|incident|broke|outage' -i --since=18.months
```

**Match the alternation syntax to the flag.** `--grep` defaults to basic regex, where alternation
is `\|`; `-E` selects extended, where it is a bare `|`. Both work when they agree — measured on one
repo, `--grep='fix\|feat'` and `-E --grep='fix|feat'` each returned 13. **Mixing them returns
zero**, silently, which reads as *"this repo has no incidents"*.

The safe habit is to write `-E` and a bare `|`, and to confirm the pattern matched a commit you
know exists before recording a zero.

`--grep` searches the **whole message**, not just the subject, which is what you want here: a
revert's diagnosis usually sits in the body.

Two shapes are worth more than the rest:

- **A revert.** Something passed review and shipped wrong. Read the reverted diff: whatever
  reviewers missed is the probe.
- **A file fixed more than twice.** Read the fixes together. Repeated fixes to one file almost
  always share an unnamed invariant, and that invariant is an L6(a) question — *is it asserted
  anywhere it can be re-violated?*

```
git log --format='%h' --since=18.months -- <file> | wc -l
```

## Source 3 · Decision and report directories

```
git ls-files 'docs/decisions/**' 'docs/adr/**' 'docs/reports/**' 'docs/postmortems/**' 'rfcs/**'
git grep -lE '^#+ *(Consequences|Decision|Status)' -- docs
```

An ADR's **Consequences** section is a list of things that go wrong if the decision is forgotten
— which is the definition of a lens probe. **Take the consequence, not the decision.**

Where a report records a measurement, keep the number **and its units**: a measurement is a fact
with a stopwatch behind it and it survives. A forecast is not, and no duration ever appears in the
emitted skill.

## Source 4 · The tracker

```
gh issue list --state closed --label bug --limit 60 --json number,title
gh pr list --state merged --search 'revert' --limit 30 --json number,title
```

Lowest yield per byte — titles are terse and the diagnosis lives in comments. Use it to **confirm**
a pattern the first three sources suggested, not to discover one.

---

## Classifying what you found

Every incident lands under exactly one lens, and the lens decides its wording.

| If the incident was… | Lens | The question it makes concrete |
|---|---|---|
| built in the wrong unit / plane / layer | L1 | is this the repair site, or did the defect enter upstream? |
| a reimplementation of something that shipped | L2 | which existing seam owns this? |
| a guard raised, widened, or that lost a flag | L3 | which direction did that number move? |
| something that existed and affected nothing | L4 | can this assertion fail? |
| a hop never connected | L5 | does the other end exist, in **both** directions? |
| the second row, the failing element, the abort, the re-run | L6 | which of the four questions? |

**L6 is worth a deliberate second look** — its shape is the one that survives review, because the
hop existed and the happy path worked, and what failed was the second item, the concurrent run, or
an exit path nobody enumerated.

**An empty L6 is a legitimate result.** Before recording it, confirm the search was sound — run
source 2 with `-E` and check the pattern matched a commit you know exists — and then say so
plainly rather than digging until something turns up:

```
L6: no incident found in this repo's history; the lens ships its four portable questions only
```

Mining until an incident appears is how a generator invents one. Absence is a finding here too.

The canonical shape, worth recognising: one defect re-entering through **four different doors**
over four separate fixes, because no fix named the invariant. Each fix was correct. None of them
wrote down the rule, so the next state added re-broke it. That is why L6(a) asks the author to
state the invariant in one sentence, and then asks where it is asserted.

## Writing an incident into the emitted skill

```
### {{the question it makes concrete}}

> {{one sentence: what happened, past tense, with the cost if it was recorded}}

Check: {{the generalised form — what to look for on a diff, not the specific bug}}
```

- **One sentence, past tense, concrete.** *"A queue mint stopped after one row: a 128-byte id cap
  the prefix nearly exhausted — row 1 fit, row 2 did not."*
- **Generalise the check, never the story.** The story is what makes it get read; the check is
  what makes it get used. Keep them separate.
- **Scrub anything that should not travel** — customer names, credentials, internal hostnames,
  people's names. An incident is a shape, and the shape survives anonymisation intact. Where the
  generated skill will live in a public repo, this is not optional.

**No stored counts.** An incident may say what happened; it may not assert how many of something
exists in the tree today. That number goes stale silently, and a reviewer carrying a stale
inventory is the precise defect this whole design exists to catch.
