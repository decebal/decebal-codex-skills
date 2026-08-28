# Debugging discipline

## Instrument before theorizing

If you have a bug you can't explain and a guess at the cause, write the log line
FIRST — one that captures the exact event chain (flags, target, payload, stack
trace, timestamps) — then read it. Theorizing for an hour without evidence is the
most expensive failure mode there is. If a log server or a tail tool is available,
use it; do not reason about what the log *would* say.

## Revert before layering

If a "fix" doesn't fix the bug, revert it BEFORE trying the next fix. Layered
guesses are how a five-minute regression becomes a four-hour session that breaks
more than it fixes. Worth re-saying: when an attempted fix fails,
`git checkout HEAD -- <file>` first, then try the next idea.

## Identify the regression commit

If the user says "this worked before recent changes", run `git log --oneline -- <file>`
and inspect the most recent commits that touched the surface. The
`git show <hash> -- <file>` diff usually contains the smoking gun — especially for
popovers, event handlers, and async state.

## Confirm which condition failed, not which condition exists

A refusal or an error usually covers a compound condition. Reading the code tells
you the shape of the gate; only the runtime state tells you which conjunct was
false. See [evidence-discipline.md](evidence-discipline.md).

## Renderer and embedded-webview gotchas

Desktop webviews are not browsers, and the differences show up as timing bugs:

- A `requestAnimationFrame` loop positioning an overlay above a draggable
  title-bar region produces delayed, `isTrusted: true` clicks at the trigger
  coordinates — a phantom second click. Ban `autoUpdate({ animationFrame: true })`
  and RAF-driven positioning outright; reposition on scroll/resize events instead.
- Clipboard APIs can silently no-op. Wrap the platform call and use the wrapper
  everywhere.
- A blur/reveal wrapper that sets `filter` or `transform` on an ancestor breaks
  `position: fixed` for everything inside it. Overlays must not be nested under
  one.

Record the ones your stack hits, with the symptom first — the symptom is what a
future session will search for.
