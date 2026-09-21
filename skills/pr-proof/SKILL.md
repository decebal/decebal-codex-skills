---
name: pr-proof
description: "Build the HTML evidence report a PR ships — the claim, before/after anchored to commit SHAs, screenshots with provenance, copy-paste manual test steps, and what was NOT checked. Use when a PR changes a user-visible surface, when a reviewer asks to be shown, or when someone says prove it, proof, evidence report, PR report, before and after, or hand-check. Produces one self-contained file under docs/, committed in the branch and linked from the PR body."
---

# PR proof

`rules/pr-evidence-report.md` states the contract. This skill produces the artifact.

Codex loads `AGENTS.md` and does not expand Markdown `@` imports, so this file is
read directly rather than pulled in by reference. Every path below is explicit
for the same reason.

A diff shows what changed. It cannot show **what a person can now do that they
could not before**, and it cannot show that anyone watched it happen. Gates prove
the code compiles and the assertions pass; a green gate has never demonstrated a
feature. The report is where that evidence goes.

## Before you build anything

**Ask whether the report is earned.** It is earned by having evidence to show —
never by policy. Write Markdown and stop unless the change needs at least one of:

1. Screenshots whose captions must stay attached to them.
2. Collapsible sections, because there is superseded evidence or long output.
3. Copy buttons, because a reader will *execute* the manual steps.
4. Before/after side by side at equal height.

A refactor with no user-visible change needs none of those.

## Gather first, write second

Do not open the template until you can fill these. Each one you cannot fill is
either a gap to close or a line for the limits section.

| Slot | How to get it |
|---|---|
| **The claim** | One sentence naming the end-to-end thing a person can now do. If it cannot be stated as a complete job someone finishes through the real flow, it is not a claim yet. |
| **Before SHA** | `git merge-base origin/<trunk> HEAD` |
| **After SHA** | `git rev-parse HEAD` |
| **Screenshots** | Drive the real running product. Capture the window, not a mock, a fixture harness or a dev-server page. |
| **Provenance per shot** | Process id, window id, timestamp, which binary, which profile or tenant. A screenshot without it is decoration. |
| **Manual steps** | Every command verified to exist **this session**. Never forward a command from a stale runbook. |
| **Pass criterion** | Per step, observable. "Looks right" is not one. |
| **Traps** | Failure modes that look like your change and are not. Each with its tell. |
| **Limits** | What you did not check. Always non-empty in practice. |

**A capture you could not take is a `capture-gap`, not silence.** The template
has a bordered placeholder. Omitting a shot reads as "nothing to see"; the gap
reads as "not established", which is the truth.

## Build it

1. Copy `assets/report-template.html` to
   `docs/reports/YYYY-MM-DD-<slug>.html` — or `docs/checklists/` for a
   hand-check, `docs/runbooks/` for a procedure. Follow the repo's existing
   `docs/` layout; do not invent a folder.
2. Put images in `docs/<kind>/assets/<slug>/` beside it. Commit them.
3. Fill the seven sections **in order**. The order is the argument: claim →
   evidence → limits, never the reverse.
4. Headings are **claims, not labels**. "Stop is where the words are" beats
   "Streaming behaviour". A heading that could sit above any section of any
   report is a wasted line — the reader skims headings, so put the finding there.
5. Link it from the PR body.

## The seven sections

| # | Section | Holds |
|---|---|---|
| 1 | The claim | One falsifiable sentence. Not "improves X" — that asserts nothing. |
| 2 | Before / after | Behaviour first, code only where the code *is* the point. Each panel labelled with its SHA. |
| 3 | What a person actually sees | Screenshots, each captioned with provenance. |
| 4 | Prove it yourself | Numbered steps, every command in a copy block, every step ending in **Pass:**. |
| 5 | Traps | The failure modes that mimic your change. |
| 6 | What this proves that the tests cannot | If you cannot fill this, you did not need a report. |
| 7 | What I have NOT checked | Always present. Always last. Never softened. |

## Three rules that decide whether the report is honest

**Never collapse the claim, the limits, or a failure.** `<details>` holds
secondary and superseded evidence — a capture a later one replaced, long raw
output, a remaining-checks list. A reader who expands nothing must still come
away with the correct impression, including the bad parts. Collapsing a caveat is
how a report lies without containing a false sentence.

**`data-copy` and the `<pre>` must stay byte-identical.** They drift the moment
someone edits one, and the copied command then differs from the reviewed one.
One command per block — a reader pastes blocks, they do not parse them.

**"Before" without a SHA is unverifiable.** The past is where a
plausible-and-wrong story is cheapest to tell, so anchor both panels.

## Checks before you link it

- [ ] Opens from `file://` with no network — inline `<style>`, inline `<script>`, no CDN
- [ ] Every `data-copy` matches its `<pre>` exactly
- [ ] Every command was verified to exist this session
- [ ] Every screenshot caption carries provenance
- [ ] Section 7 is present, last, and not empty
- [ ] Nothing load-bearing is inside `<details>`
- [ ] Renders at phone width; `prefers-reduced-motion` respected
- [ ] Committed in the branch, not pasted into a comment or hosted elsewhere

## Where it does not go

Not a hosted page, not a gist, not a chat attachment. A report outside the repo
is reviewed by nobody, found by no future reader, moves with nothing, and leaves
no history when deleted. See `rules/documents-not-artifacts.md`.
