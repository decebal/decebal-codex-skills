# PR evidence report — code samples and examples

Detailed code samples and incident documentation supporting `rules/pr-evidence-report.md`.

## HTML code samples

### Screenshot with provenance

```html
<figure id="shot-desktop">
  <a href="assets/<slug>/desktop.png"><img src="assets/<slug>/desktop.png"
     alt="<factual description of what is on screen>" loading="lazy"></a>
  <figcaption><strong>PID 44555 · window 9215 · 19 September, 00:12:30 UTC.</strong>
    Captured from the workspace's <code>target/debug/&lt;binary&gt;</code>.</figcaption>
</figure>
```

### Before/after side by side

```html
<div class="ba">
  <div><h4><span class="tag bad">Before</span> <sha></h4><pre>…</pre></div>
  <div><h4><span class="tag good">After</span> <sha></h4><pre>…</pre></div>
</div>
```

The SHA is what makes it checkable. "Before" without one is a claim about the past that nobody can verify, and the past is exactly where a plausible-and-wrong story is cheapest to tell.

### Copy block with fallback for clipboard failure

```html
<div class="copy"><button class="cp" data-copy="<exact command>">Copy</button><pre><exact command></pre></div>
```

- **`data-copy` and the `<pre>` must be byte-identical.**
- **One command per block.** A reader pastes blocks; they do not parse them.
- **Every step ends with `Pass:`** naming an observable outcome. "Looks right" is not a criterion. "`tool_ready_connections=` lists your new server" is.
- **Never fabricate a command.** Verify it exists this session.

JavaScript fallback when clipboard permission is refused:

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

### Print stylesheet

Make the print stylesheet expand `<details>` so a printed or PDF'd report is complete:

```css
@media print { details > div { display: block } }
```

### Self-contained accessibility

- A skip link, focus-visible outlines, and `scroll-padding-top` if the nav is sticky.
- Responsive down to phone width.
- `prefers-reduced-motion` respected.
- `loading="lazy"` on images.

## The convention regression, 2026-09-21

Across the last 20 PRs in one repo, five shipped HTML reports. All five used `<section>` + `<h2>` and an inline `<style>`; four had copy buttons. But **`<details>` appeared in only two, and screenshots in only two** — so the two things that most distinguish a report from a long comment were present in under half of them, and no two reports put their sections in the same order.

The convention was real and working, living entirely in whichever file the author happened to copy. That is the failure this rule exists to stop: not absent practice, but practice that cannot be inherited.
