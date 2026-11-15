# Spec

Write the design, have it reviewed by someone who did not write it, and get a human to approve
it. Output: `docs/plans/YYYY-MM-DD-<slug>-design.md`, its two reviews beside it, committed on the
flow's branch, and an approved epic.

## The branch

Cut the flow's branch now, in its own worktree, and name it on the epic. The design, its reviews,
the plan and the code all land on it and merge as one PR:

```bash
git fetch origin <trunk>
git worktree add -b <type>/<slug> ../<repo>-<slug> origin/<trunk>
cn task edit <epic> --append-description "ritual-branch: <type>/<slug>" --toon   # from the main checkout
```

Use the repo's branch rule (a ticket key, see the org's notes in `repo-notes.md`). Move the Think brief from the main
checkout onto the branch, and commit the design there as you draft it: `cos` reads the committed
copy while the branch exists, and a hired reviewer's worktree is cut from it.

## Draft (the Elephant)

The author has the full context and drafts here. EGM's `/egm:egm-draft` (writes
`design_draft.md`, moved to the dated path) runs only when the user asks for EGM
([Opt-in: EGM](../SKILL.md#opt-in-egm)).

A design covers:

- **Decision** first, in one paragraph.
- **Context:** the system as it is, with file paths.
- **Design:** components, data flow, interfaces and their shapes, error handling.
- **Alternatives** rejected and why.
- **Security and privacy:** what is read, what is stored, what leaves the machine.
- **Testing:** which layers prove it (see the testing trophy in `plan.md`).
- **Confidence:** each piece as rock, sand or water, and what moves it up. Never a duration.
- **Acceptance:** observable checks a reviewer can run.

Write requirements as what must be true, not as an implementation. Name terms once and use them
consistently. Mark every assumption.

Name the design on the epic:
`cn task edit <epic> --append-description "ritual-design: <path>" --toon`.

## Review (the Goldfish)

A reviewer reads only the draft and the live code, never the author's conversation. A reviewer
that read the chat inherits its blind spots.

Start two reviewer agents with `spawn_agent`, each in a fresh context (or a fresh `codex exec`
session), each given only the design path and the repo root and told to check every claim against
the code. They never see each other's output, and each writes its own review file with a verdict
line. A `cos`-hired `ritual-goldfish` may be one of the two. EGM's `/egm:egm-review` stands in
only when the user asks for EGM. `allstack spec review` replaces the two agents once it is merged
in allstack source and the binary is rebuilt.

If no fresh context can start, stop: do not review in the author's context.

Each review is `docs/plans/YYYY-MM-DD-<slug>-design-review-<a|b>.md`:

```markdown
---
design: docs/plans/YYYY-MM-DD-<slug>-design.md
design-sha256: <sha256 of the design file the reviewer read>
verdict: clean | findings
---

1. <blocking | should | nit> <section>: <problem>. Evidence: <file:line or command>. Fix: <fix>.
```

A review with no `verdict:` line is not a review: send it back. On the EGM path, its
`goldfish_review_report.md` has none, so record its outcome in this file, `clean` only when it
raised nothing blocking or should-fix.

A hired `ritual-goldfish` works from a worktree cut at a commit, so commit the design on the flow's
branch before hiring it; its review stays in the agent's output, and you point `approve` at it.

Approval needs both reviews `clean` against the same digest. Resolve findings in the design, then
run both again. Each review records the digest it read; an edit after a clean review needs new
reviews. Keep earlier reviews as history and mark their `design-sha256: superseded`.

## Approve

The approval is the user's decision. Show the user the design and both clean reviews, then wait.
Two ways it happens:

- the user runs the command in their own terminal:

  ```bash
  cos ritual approve <epic> docs/plans/YYYY-MM-DD-<slug>-design.md --by "<name>" \
    --review docs/plans/YYYY-MM-DD-<slug>-design-review-b.md
  ```

  `--review` takes the later of the two clean reviews; the other is named in the design's review
  history.

- the user tells you to approve in this conversation. First run `cos ritual approve --help` and
  check that it lists `--yes`; if it does not, stop and tell the user to update chief-of-staff-v2 to
  main. Do not run the command without `--yes`: it asks for a terminal answer you cannot give.
  If it does, run the same command with `--yes` and quote the user's instruction in your report.
  Never approve on your own initiative, never infer approval from silence or from a clean review,
  and never approve to unblock yourself.

It refuses unless the review names this design, its `design-sha256` matches the design's current
digest and its verdict is `clean`. Without `--yes` it asks for confirmation and refuses without a
terminal. Then it appends `ritual-approved: <path> sha256:<digest> by <name>` to the epic and runs
`cn approve <epic>`. `--without-review` replaces `--review` for designs too small to review, and
says so in the line it writes. `--by` is a name the caller types, not a verified identity.

Without `cos`, the user computes `shasum -a 256 <design>`, appends the same line, and runs
`cn approve <epic>`; if the user tells you to, you run `cn approve <epic>` after that line exists.

Then the next step is Plan ([plan.md](plan.md)).
