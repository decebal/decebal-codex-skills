---
paths:
  - "docs/**/*.html"
  - "docs/**/assets/**"
  - "**/.github/pull_request_template.md"
---

# The PR evidence report

Portable. Assumes a repo with a `docs/` tree and a forge that renders links.

A diff shows what changed. It cannot show **what a person can now do that they
could not before**, and it cannot show that anyone actually watched it happen.
That is the report's job, and nothing else in the pipeline does it: gates prove
the code compiles and the assertions pass, and a green gate has never once
demonstrated a feature.

**Every PR that changes a user-visible surface ships one HTML report, committed
in the branch under `docs/`, linked from the PR body.** Not a hosted page — a
file in the repo, reviewed in the diff like everything else, moving with the
code it describes. See [documents-not-artifacts.md](documents-not-artifacts.md).

## Why HTML and not Markdown

Markdown is the default for a document. This is the exception, and only for
these four reasons:

1. **Screenshots with captions that stay attached.** A `<figure>` keeps the
   image and its provenance together; Markdown separates them the moment
   anything reflows.
2. **Collapsible sections.** `<details>` lets a report carry superseded
   evidence and long output without burying the claim. Markdown has no
   equivalent that survives outside the forge.
3. **Copy buttons.** A manual test step is executed, not read. One click beats
   a careful triple-click through a wrapped line.
4. **Before/after side by side.** Two panels at equal height, each labelled
   with a commit, is a layout — not something a fenced block does.

If a change needs none of those, write Markdown and stop. A report is earned by
having evidence to show, never by policy.

## The seven sections, in order

Order is the argument. It runs claim → evidence → limits, never the reverse.

| # | Section | Holds |
|---|---|---|
| 1 | **The claim** | One sentence naming the end-to-end thing a person can now do. Not "improves X" — that asserts nothing and cannot be falsified. |
| 2 | **Before / after** | The behaviour, paired. Each panel labelled with its **commit SHA**. |
| 3 | **What a person actually sees** | Screenshots of the real running thing, each captioned with its provenance. |
| 4 | **Prove it yourself** | Numbered steps, every command in a copy block, every step ending in an explicit **Pass:** criterion. |
| 5 | **Traps** | The failure modes that look like your change and are not. Each with its tell. |
| 6 | **What this proves that the tests cannot** | The reason the report exists. If you cannot fill this, you did not need a report. |
| 7 | **What I have NOT checked** | Always present. Always last. |

**Headings are claims, not labels.** "Stop is where the words are" beats
"Streaming behaviour". "One stage armed two timers for the same instant" beats
"Timer fix". A heading that could sit above any section of any report is a
wasted line — the reader skims headings, so put the finding there.

**Section 7 is not optional and does not get softened.** A report without a
limits section is marketing. Observed headings that do this job well: *"What I
have not checked."*, *"What was read, and what remains unknown."*, *"What is
actually proven"*.

## Screenshots carry provenance or they prove nothing

A screenshot with no provenance is a picture. The caption states **what produced it**: process id, window id, timestamp, which binary, which profile or tenant.

**Detail:** `~/.codex/rules-reference/pr-evidence-report-examples.md` — HTML code samples (screenshot `<figure>`, before/after layout, copy blocks with fallback), CSS, accessibility requirements.

- **`alt` describes the screen, not the intent.**
- **Assets go in `docs/<kind>/assets/<slug>/`**, beside the report, committed.
- **A screenshot you could not take gets a visible gap**, never silence.
- **Never screenshot a mock, a fixture harness or a dev-server page.**

## Before / after is anchored to commits

The SHA is what makes it checkable. "Before" without one is a claim about the
past that nobody can verify. Show **behaviour** before code. A user-visible before/after — the sentence that changed, the card that appeared — is worth more than a diff the reviewer can already read in the Files tab.

## Copy blocks: verbatim, one command, with a Pass line

- **One command per block.** A reader pastes blocks; they do not parse them.
- **Every step ends with `Pass:`** naming an observable outcome.
- **Never fabricate a command.** Verify it exists this session.

## What goes inside `<details>`, and what never does

Collapsed content is **secondary or superseded** evidence: a capture the later
one replaced, long raw output, the remaining-checks list, an excluded artifact.

**Never collapse the claim, the limits section, or a failure.** A reader who
expands nothing must still come away with the correct impression, including the
bad parts.

## Self-contained, always

One file. Inline `<style>`, inline `<script>`, no CDN, no build step, no
framework. It must render from `file://` years from now, on a machine with no
network, after the toolchain that made it is gone.

Also non-negotiable: skip link, focus-visible outlines, responsive down to phone width, `prefers-reduced-motion` respected, `loading="lazy"` on images.

**Detail:** `~/.codex/rules-reference/pr-evidence-report-examples.md` — the 2026-09-21 convention regression that made this rule necessary.
