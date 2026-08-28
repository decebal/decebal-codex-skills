# Evidence discipline

Two habits, one idea: **look at the artifact, not at a number reporting on it.**

## Check the DESTINATION before you trust an ABSENCE at the source

An absence never carries its own explanation. The same empty output means "your
work is gone" and "your work arrived and the source was tidied up", and those
demand opposite responses — panic and re-land, versus nothing at all.

**So the order is fixed: confirm the artifact exists at the DESTINATION first, and
only then interpret what is missing at the source.** Never the reverse. Read in
the wrong order, an empty result at the source frames everything after it, and the
reading you reach for is the alarming one.

The canonical case: after a PR merges, the forge deletes the branch, so
`git ls-remote origin <branch>` returns **empty** — byte-identical to a push that
never landed.

```bash
# WRONG ORDER — the empty result invites the alarming reading
git ls-remote origin "$B"                        # empty. Lost push? Deleted branch?
# → a needless re-land, or a false "your work is gone"

# RIGHT ORDER — establish the destination, then the absence is bookkeeping
git show origin/main:<path> | grep <the-change>  # is it THERE? ← answer this first
git ls-remote origin "$B"                        # empty now MEANS "cleaned up"
```

The pattern generalizes past git: **a proxy signal standing in for the artifact.**

| Absence at the source | Innocent meaning | Alarming meaning |
|---|---|---|
| `ls-remote` empty | branch auto-deleted on merge | push never landed |
| push exit code `0` | it worked | a trailing `echo`/pipe owns the code, the gate FAILED |
| empty `grep`/`diff` output | genuinely not there | filtered, wrong ref, or rewritten by a wrapper |
| log file 0 bytes | nothing has happened yet | output buffered, not a tty |
| test filter matched nothing | no such test | the suite never ran |
| `pgrep` count 0 | idle | wrong pattern (`-f` self-matches; use `-x`) |
| zero rows returned | no such data | you queried the wrong store |

Every right-hand column is a real incident. In each one the fix was the same: **go
look at the thing itself** — the remote ref, the file content on the target ref,
the log bytes, the process list. An exit code, a match count and a process count
are all claims *about* the artifact; only the artifact is evidence.

**A test that passes by never running is the same failure** wearing a green badge.
See [testing-gates.md](testing-gates.md).

## Read runtime and store state — never guess it

Any claim about a running system or a data store — a toggle's value, a config
"default", an allowlist, whether something is connected, **who changed what** —
MUST come from the actual data. Never from memory, inference, or "it defaults to
X". Guessing here produces confidently-wrong statements. If you are about to state
a runtime fact you did not just read, stop and read it.

- **Answer from the store that HOLDS the answer.** Most systems have more than one
  — a local/per-machine store and a hosted/per-tenant one — and they are not
  interchangeable. A question about the tenant is not answerable from the local
  store, and a question about this machine is not answerable from the hosted one.
  **Querying the wrong one returns an empty result that reads exactly like "no such
  data".** Say which store you read.
- **If you have access, do not deny it.** When the tools or the files are present,
  never say you "can't reach" them or that the value is "inaccessible" — go read
  it. This is a rule about not *ducking* the work, not a claim that every session
  has the store: a cloud, headless, or fresh-machine session may genuinely have
  neither. If so, say which store you tried and what was missing. An honest "that
  store isn't on this machine" is correct; "I can't access your data" when it is
  sitting there is not.
- **A data-access tool can be bound to the WRONG or STALE store.** Run a cheap
  stats/health probe first and check the date range and row count. Stale (old dates,
  tiny counts, missing record types you expect) means the tool is pointed at a dev
  or archived copy — do NOT conclude "no records exist". Find the live path and
  read it directly.
- **File size means nothing for a write-ahead-log-backed store.** A 0-byte
  database file with the data in adjacent segment files is normal. Read the
  segments.
- **Fold the history to get current state.** Reading one record and stopping gives
  you a moment, not the value. Last-writer-wins by timestamp.
- **Don't assert cause from a code path you skimmed — confirm which condition
  actually failed.** A refusal message covering one gate with two conjuncts
  (`allowed = feature_on && domain_permitted`) tells you nothing about which
  conjunct is false. Read the state for each. A real case inverted the guess
  entirely: the feature had been on for months; the allowlist was the miss.

## A model's words are a CLAIM, not evidence

When an agent, a subagent, or a log line reports what it did, that is testimony.
Tabulate the actual calls, diffs, or rows before repeating it as fact — especially
before the claim reaches someone who will make a decision on it.
