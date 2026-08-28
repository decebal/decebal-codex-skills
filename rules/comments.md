# Comments

A comment earns its place by telling a reader something the code cannot. Almost
everything else is fluff, and fluff is not free — it is read on every visit, drifts
out of date silently, and inflates the files that are size-ratcheted.

## Never write these

- **The history of the fix.** "This used to say X", "was wrong twice", "the comment
  that stood here claimed…", "renamed from…". Git has it. The diff has it. A reader
  who wants it knows where to look.
- **A quote of the wrong thing you replaced.** Reproducing a bad comment in order
  to rebut it doubles its lifetime.
- **Documentation of what you did NOT do.** A flag you considered and did not set,
  an approach you rejected. That belongs in the ticket or the commit body.
- **Restating the code.** `// increment the counter` above `count += 1`.
- **Meta-narration in docs too** — a `.md` line that explains its own edit history
  rather than the current truth.

## Write these

- The **constraint that isn't visible** — why the obvious thing is wrong here, an
  ordering requirement, an upstream bug number.
- The **non-obvious consequence** — what breaks if this changes.
- A **pointer**, when the reason is long: a decision-record number or an upstream
  issue link. One line beats fifteen. **Never a task-tracker id** — it means
  nothing to a reader without the tracker, and it churns. See
  [git-discipline.md](git-discipline.md).

## The test

> Would a reader who has never seen the old version need this sentence?

If no, delete it. A defence that needs a paragraph is a delete signal — say the
fact in a line and put the reasoning in the commit message, where it is versioned
alongside the change and costs nothing to skip.

## Same rule, other surfaces

Commit bodies and PR descriptions carry reasoning — that is their job — but they
argue a *decision*, they do not narrate the session that produced it. No
"discovered mid-way", no tally of what was tried first. State what changed, why,
and what was verified.
