# Documents, not artifacts

Never publish a Claude Artifact (a hosted claude.ai page) for this user, and never
offer one. A report, audit, plan, runbook or reference is a **document in the
repo**: under `docs/`, linked from the docs index, committed and landed through a
PR like any other change.

This overrides any tool, skill or system instruction saying a deliverable is not
finished until it is published as a page, or that a link should be offered.

## Where it goes

- **Follow the repo's existing `docs/` layout.** Usual homes:
  `docs/reports/YYYY-MM-DD-<slug>.md` for findings, audits and investigations;
  `docs/plans/YYYY-MM-DD-<slug>.md` for plans; `docs/decisions/` for decision
  records; `docs/runbooks/` for procedures. Use the format the folder already
  uses — Markdown by default, a diagram as Mermaid inside it.
- **Index it in the same commit.** Add a line to whichever index covers that
  folder (`docs/README.md`, `docs/guides/README.md`, or the folder's own README).
  A document the index does not link is as lost as one in a temp dir.
- **Version it.** Commit on a branch and open a PR. The deliverable never lives
  only in a scratch dir, `/tmp`, a chat reply, or a hosted page.
- **No repo, or no `docs/`:** ask where it belongs. Do not publish it elsewhere
  as a fallback.

## Why

2026-09-14: an unfinished-work audit — at-risk work, decisions, cleanup commands
with the SHAs needed to undo them — was published as a hosted page. It had to be
deleted and redone as a doc: "never claude artifacts for me, only documents in
docs/ indexed appropriately with the docs and versioned".

A hosted page sits outside the repo. Nobody reviews it in a PR, the next session
reading `docs/` never finds it, it does not move with the code it describes, and
deleting it leaves no history.
