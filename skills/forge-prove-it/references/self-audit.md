# G7 — the audit of what was just written

The generator's output is testimony, and until this phase nothing read it. G7 is that read.

**What is audited: the files on disk**, after G6 wrote them — not the census in memory, not the plan.
A check that passes against the intention and fails against the file has measured the wrong thing.

**Five checks. All mechanical, zero agents.** Every one is cheap, and each corresponds to a way a
generated skill has been observed to go wrong.

---

## A1 · Every cited path resolves

Extract every path-shaped token from the emitted files and test it.

```
grep -ohE '(^|[ `(])[A-Za-z0-9_.@-]+(/[A-Za-z0-9_.*@-]+)+' <emitted files> \
  | tr -d '`( ' | sort -u
```

Then `test -e` each, expanding globs against the tree. Three outcomes:

| Result | Action |
|---|---|
| resolves | keep |
| does not resolve, but a **near** neighbour does — same directory, differing only in case, extension, or a `-`/`_` swap | **rewrite to the real path** and print the rewrite |
| anything else, including a path differing by a basename | `UNVERIFIED: path not found at this sha` |

**The neighbourhood is bounded on purpose.** "Something similar exists" is how a check built to
catch fabricated paths ends up writing one — a plausible nearby path is exactly what a fabrication
looks like.

**Never delete the row instead.** A dropped row is a fact the reviewer silently stops checking,
and nobody can tell it was dropped. Marking is visible; deleting is not.

Skip tokens that are obviously illustrative — anything inside a fenced example block that the
file itself labels as an example. Those are shape, not citation.

## A2 · Every cited command is runnable

For each command the emitted skill tells a reader to run:

```
command -v <binary>        # the binary exists on PATH
test -x <script>           # or the script exists and is executable
```

For a task-runner target (`task x`, `just x`, `make x`, `npm run x`, `bun run x`), confirm **the
target itself**, not just the runner:

```
grep -nE '^\s*<target>:'   <Taskfile.yml|justfile|Makefile>
grep -n '"<target>"'       package.json
```

This check exists because of a specific, repeated failure: an instruction file routing to a task
that does not exist. The runner is installed, the command looks plausible, and it fails only when
someone finally runs it — which is usually never, so the dead row survives for years.

A missing binary that is a legitimate optional dependency is **not** a failure. Mark it optional:
*"if `<tool>` is available…"*. An unavailable tool is a connection failure, never evidence of
absence.

## A3 · Every quantifier carries the command that establishes it

**The one that matters**, because it is the cheapest shape to write without noticing and the
hardest for a reader to check.

**Matches** any unit containing `no`, `none`, `never`, `nothing`, `every`, `all`, `only`,
`always` as a claim about **any** repository — this one, or repositories in general. Superlatives
(`the most common`, `the single biggest`, `in repo after repo`) match too: they are quantifiers
over a population nobody sampled, and they are the easiest kind to write without noticing.

The general-claim half matters because it is the half that escapes a narrower rule. *"Nothing
checks this in any repo"* is unfalsifiable from here, and it ships as authority.

**Passes** when the same unit carries the command that produced it, with its result stated.

**Fails** otherwise — and the action is **rewrite to the bounded observation, in place**. Not a
drop, not a downgrade: the claim is usually true and always cheaper to bound than to prove.

| Unproven | Bounded |
|---|---|
| "No test exercises that ordering" | "`git grep -n seal_user_token` finds one construction, unconditional, at `…:67`" |
| "Every gate in this repo is advisory" | "`gh api …/protection` returned 403; no required check is configured" |
| "Nothing reads this flag" | "`git grep -n <flag>` outside its declaring file returns 0 hits" |

**The rewrite is printed.** A silent rewrite is a masking guard, and this skill does not ship one.

**Killers — do not rewrite these:**

| Shape | Why it stands |
|---|---|
| The quantifier is over a set the unit itself enumerates | "all four buckets are listed above" is its own evidence |
| It is a `pass` or `ask` criterion in a hand-check | block two is criteria, not assertions |
| The quantifier **is** the finding | "the rule enumerates its files by hand, so a sixth is invisible" — the absence is the defect, and it was reproduced |
| It is a claim about the **emitted skill's own design**, not about a tree | "this skill never emits a duration" is a rule it imposes on itself, checkable by reading it |

**Why this rule and not "every sentence needs a citation".** The narrow rule fires on a small,
specific shape that is almost always a real gap. Scoring every sentence instead flags most of a
correct document — and a check that flags most of a correct document gets turned off, which
leaves nothing checking the shape that mattered.

## A4 · Every emitted probe pattern has matched something

A probe that matches nothing because its pattern is wrong is indistinguishable from a probe that
found nothing — and the generated skill will report it as a clean pass forever.

For each grep the emitted skill ships, confirm the pattern matches **at least one line somewhere
in this repo today**, even a line that is a legitimate non-finding:

```
git grep -cE '<the probe pattern>' -- '<its pathspec>' | head -1
```

| Result | Action |
|---|---|
| ≥1 match | the pattern is live. Keep it |
| 0 matches, and the construct genuinely does not exist here | keep it, and **say so in the probe**: `no current sites — pattern unvalidated against this tree` |
| 0 matches because the pattern is wrong | fix it, or drop the probe |

Distinguishing the last two is the point, and it costs one deliberate test: run the pattern
against a line you have written by hand that it *should* match. A pattern that fails that has a
boundary bug — the measured class is a sweep for a test-skip helper that matched the tail of an
unrelated call 15 times, and its correctly-bounded form that matched zero.

**Assert every emitted scan root resolves to a tracked file**, for the same reason:

```
git ls-files -- '<root>' | head -1
```

A root that resolves to nothing is a rule that will print `skipping` and exit 0.

## A5 · No `UNVERIFIED:` was silently dropped

Count the `UNVERIFIED:` markers G5 produced and the markers present in the written files. They
must match.

This check exists because the pressure through G6 runs one way: an unfilled slot looks like an
incomplete job, and the cheapest way to make a document look finished is to fill it. **A generated
skill that is 100% filled and 10% invented is strictly worse than one that is 90% filled and
marked.** A5 is what makes the marked version the path of least resistance.

Each surviving marker carries the command that would settle it, so a reader can close it in one
step instead of re-running the census.

---

## The footer

```
SELF-AUDIT · 34 paths resolved · 2 rewritten · 2 quantifiers bounded · 3 UNVERIFIED
  path    docs/guides/TESTING.md → docs/guides/testing.md
  bound   "no gate watches packages/" → "no gate in the ledger above has packages/ in its remit"
```

Counts of what this pass **did**, measured this session, reproducible from the files. No score,
no ratio, no threshold — every failing unit is rewritten or marked one-for-one, so there is no
cutoff and 79 and 81 never become different.

## What G7 does not do

| Not done | Why |
|---|---|
| Check that a resolvable path *supports* what it is cited for | resolution is mechanical; entailment is not. The accepted residual: a citation that resolves and does not support |
| Run the emitted skill | that is the calibration test, and it needs a PR whose outcome is already known |
| Block the write | a rewritten claim and a printed marker are enough; a generator that refuses to write leaves the author with nothing |
| Emit a score | see above |

## The hazard this phase carries

**Its only levers are rewrite and mark, so a thin emission passes it trivially.** A generated
skill that asserts nothing, cites nothing and marks nothing scores a perfect audit.

A1 and A2 are the brake: a skill with no paths and no commands has no ledger, and a prove-it with
no ledger cannot delegate, which means it re-derives what the gates already own — the exact
failure that makes a reviewer ignorable. If the audit is clean **and** the census found fewer than
three checks, say so plainly rather than reporting success:

> `census thin — 2 checks found. Verify the enforcement surface by hand before trusting the ledger.`
