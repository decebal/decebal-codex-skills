# Web rendering — DOM, layout, paint, re-render

The browser turns your changes into pixels through style → layout → paint →
composite. Two ways to make that expensive: force the pipeline to run more often
than needed (unnecessary re-renders, layout thrash), or make each run do more work
(large trees, late layout shifts, double renders from hydration mismatch). This
checklist covers the framework-and-DOM cost that sits above raw allocation.

**Template** — fill in your framework (the React notes below generalize to any
VDOM/signals library) and your target: interaction-to-next-paint (INP) under
~200 ms, no layout shift (CLS ~0), 60 fps during animation.

| # | Pattern | Detect | Why it costs | Fix |
|---|---|---|---|---|
| R1 | Re-render from unstable references | `rg -n 'useEffect|useMemo|useCallback|memo\('`; look for object/array/function props recreated inline each render | A new `{}` / `[]` / `() => {}` every render breaks referential equality, so memoized children re-render and effects re-fire | Wrap derived objects in `useMemo`, callbacks in `useCallback`, with **correct, stable deps**; hoist constants out of the component; `React.memo` the child. Verify with the Profiler's "why did this render" — do not add memos blind |
| R2 | Effect that runs every render | `useEffect` with a missing or unstable dependency array; a dep that is a fresh object each render | Re-running an effect can trigger state → render → effect loops and redundant I/O | Give the effect a stable dep list; memoize the deps; move one-shot work to a mount-only effect (`[]`) |
| R3 | Layout shift from late content | images/embeds/ads with no reserved size; content injected after first paint | Late-arriving content pushes laid-out content, forcing reflow and a visible jump (bad CLS) | Reserve space up front: explicit `width`/`height` or `aspect-ratio`; skeletons sized to the real content; `min-height` on late slots |
| R4 | Hydration mismatch → double render | SSR markup that differs from client render; `Date.now()`/`window`/random in render; `rg -n 'suppressHydrationWarning'` | A mismatch makes the client throw away and re-render the subtree — the initial paint's work is wasted twice | Make server and client render identical output; move client-only values into an effect after mount; gate `window` access on mount |
| R5 | rAF vs layout ordering | DOM writes scattered through handlers; reads after writes (see [main-thread-patterns.md](main-thread-patterns.md) B3) | Visual updates outside a frame, or read-after-write, force extra synchronous reflows | Do visual/DOM writes in `requestAnimationFrame`; batch reads then writes; never `autoUpdate({ animationFrame: true })` for overlay positioning — reposition on scroll/resize instead |
| R6 | Rendering large offscreen content | long lists / tab panels / cards all mounted at once | Layout and paint cost scales with the whole tree even when most is offscreen | `content-visibility: auto` (+ `contain-intrinsic-size`) to skip offscreen layout/paint; virtualize long lists (windowing); lazy-mount hidden panels |
| R7 | Overly broad state → wide re-render | a top-level context/store whose every change re-renders a big subtree | One unrelated field change repaints half the app | Split context by update frequency; select narrowly (subscribe to the slice, not the store); colocate state with the component that owns it |

## Confirm, don't guess

- Use the framework profiler (React DevTools Profiler, or the equivalent) to see
  **which** components rendered and why, and the DevTools Performance panel for the
  actual layout/paint spans. "This re-renders too much" needs the render count next
  to it.
- Measure INP / CLS with the Performance panel or `web-vitals` on a real
  interaction, before and after. A memo that does not move the measured render
  count or INP is dead weight — remove it (SKILL step 5).
- A read of layout properties (`offsetWidth`, `getBoundingClientRect`) is a
  forced-reflow marker in the Performance panel; those purple bars are where the
  batching fix from [main-thread-patterns.md](main-thread-patterns.md) B3 pays off.
