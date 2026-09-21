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

A screenshot with no provenance is a picture. The caption states **what
produced it**: process id, window id, timestamp, which binary, which profile or
tenant.

```html
<figure id="shot-desktop">
  <a href="assets/<slug>/desktop.png"><img src="assets/<slug>/desktop.png"
     alt="<factual description of what is on screen>" loading="lazy"></a>
  <figcaption><strong>PID 44555 · window 9215 · 19 September, 00:12:30 UTC.</strong>
    Captured from the workspace's <code>target/debug/&lt;binary&gt;</code>.</figcaption>
</figure>
```

- **`alt` describes the screen, not the intent.** It is read by someone who
  cannot see it, and by you in six months.
- **Assets go in `docs/<kind>/assets/<slug>/`**, beside the report, committed.
- **A screenshot you could not take gets a visible gap**, never silence — a
  bordered placeholder saying what is missing and why. Omitting it reads as
  "nothing to see"; the gap reads as "not established", which is the truth.
- **Never screenshot a mock, a fixture harness or a dev-server page and present
  it as the product.** If the real surface could not be reached, that is a
  capture gap.

## Before / after is anchored to commits

```html
<div class="ba">
  <div><h4><span class="tag bad">Before</span> <sha></h4><pre>…</pre></div>
  <div><h4><span class="tag good">After</span> <sha></h4><pre>…</pre></div>
</div>
```

The SHA is what makes it checkable. "Before" without one is a claim about the
past that nobody can verify, and the past is exactly where a plausible-and-wrong
story is cheapest to tell.

Show **behaviour** before code. A user-visible before/after — the sentence that
changed, the card that appeared — is worth more than a diff the reviewer can
already read in the Files tab. Use the inline diff spans (`del` / `add` / `ctx`)
only where the code IS the point.

## Copy blocks: verbatim, one command, with a Pass line

```html
<div class="copy"><button class="cp" data-copy="<exact command>">Copy</button><pre><exact command></pre></div>
```

- **`data-copy` and the `<pre>` must be byte-identical.** They drift the moment
  someone edits one, and the copied command then differs from the reviewed one.
- **One command per block.** A reader pastes blocks; they do not parse them.
- **Every step ends with `Pass:`** naming an observable outcome. "Looks right"
  is not a criterion. "`tool_ready_connections=` lists your new server" is.
- **Never fabricate a command.** Verify it exists this session. A runbook
  sending someone to a task that was renamed costs more than no runbook.

The button falls back to selecting the text when clipboard permission is
refused, so the block is still usable:

```js
document.querySelectorAll("button.cp").forEach((b) => {
  b.addEventListener("click", async () => {
    try { await navigator.clipboard.writeText(b.dataset.copy) }
    catch {
      const r = document.createRange()
      r.selectNodeContents(b.parentElement.querySelector("pre"))
      const s = getSelection(); s.removeAllRanges(); s.addRange(r); return
    }
    const was = b.textContent; b.textContent = "Copied"
    setTimeout(() => { b.textContent = was }, 1400)
  })
})
```

## What goes inside `<details>`, and what never does

Collapsed content is **secondary or superseded** evidence: a capture the later
one replaced, long raw output, the remaining-checks list, an excluded artifact.

**Never collapse the claim, the limits section, or a failure.** A reader who
expands nothing must still come away with the correct impression, including the
bad parts. Collapsing a caveat is how a report lies without containing a false
sentence.

Make the print stylesheet expand them, so a printed or PDF'd report is complete:

```css
@media print { details > div { display: block } }
```

## Self-contained, always

One file. Inline `<style>`, inline `<script>`, no CDN, no build step, no
framework. It must render from `file://` years from now, on a machine with no
network, after the toolchain that made it is gone.

Also non-negotiable, because a report is a user-facing surface:

- A skip link, focus-visible outlines, and `scroll-padding-top` if the nav is sticky.
- Responsive down to phone width.
- `prefers-reduced-motion` respected.
- `loading="lazy"` on images.

## The cost, 2026-09-21

Across the last 20 PRs in one repo, five shipped HTML reports. All five used
`<section>` + `<h2>` and an inline `<style>`; four had copy buttons. But
**`<details>` appeared in only two, and screenshots in only two** — so the two
things that most distinguish a report from a long comment were present in under
half of them, and no two reports put their sections in the same order.

The convention was real and working, living entirely in whichever file the
author happened to copy. That is the failure this rule exists to stop: not
absent practice, but practice that cannot be inherited.
