---
name: perf-review
description: Application-level performance review of a code change — hot paths, allocations, main-thread blocking, batching, caching, and worker offloading. Use when the user asks to "review for performance", "check perf", "is this efficient", wants a "hot path review" or a "performance audit", or before shipping code on a render loop, event handler, streaming pipeline, or request handler. Reviews app/runtime behaviour, not language semantics — the sibling typescript skill covers language-level tuning.
---

# Performance Review

Review a code change for application-level performance: work that runs often
enough that its cost matters. The language-level lane (type/compiler/async
micro-rules) lives in [../typescript/SKILL.md](../typescript/SKILL.md); this lane
is about the runtime shape of the *change* — where it runs, how often, and what it
allocates or blocks while it does.

The discipline that makes this review worth anything: **measure, do not guess.**
A performance claim without a number is a theory. Read a profile, a flame graph,
or a timing before you assert something is slow, and instrument before you
theorize about why — [../../rules/evidence-discipline.md](../../rules/evidence-discipline.md)
(look at the artifact, not at a number reporting on it) and
[../../rules/debugging-discipline.md](../../rules/debugging-discipline.md)
(instrument first). A finding that says "this allocates per frame" is only
actionable next to "and the frame budget is 16 ms, and this is 40 % of it."

Composes with [../security-review/SKILL.md](../security-review/SKILL.md) and
[../typescript/SKILL.md](../typescript/SKILL.md): all three are review lanes over
the same diff. Keep the finding format below stable so a general code review can
merge these in without re-deriving them.

---

## The method — apply to the change, in order

### 1. Identify the hot path — gate everything on this

Is the changed code frequently executed? Hot paths are: a render/animation loop,
an input or scroll or resize event handler, a streaming/media pipeline, a request
handler on a busy route, anything inside a loop over a large collection.

- **If the code is NOT hot** — startup, config load, a one-shot migration, an
  admin action — **say so and stop.** Micro-optimizing cold code adds risk and
  reading cost for no user-visible gain. "No hot-path impact" is a valid, clean
  result of this review; report it and move on.
- If it *is* hot, establish the budget before judging cost: the per-frame ms, the
  per-request p99, the events-per-second. Every finding below is measured against
  that budget.

### 2. Allocation analysis

New allocations on the hot path? Object literals, array spreads, closures created
per iteration, string concatenation in a loop. Each one is GC pressure that shows
up as jank or tail-latency spikes, not as a slow line in a profile.
→ Checklist: [references/memory-allocation.md](references/memory-allocation.md).

### 3. Synchronous blocking

Does the change block the main thread / event loop? Long synchronous compute,
sync I/O, sync crypto, `JSON.parse` of a large payload, layout thrashing from
interleaved DOM reads and writes.
→ Checklist: [references/main-thread-patterns.md](references/main-thread-patterns.md).

### 4. Batching

Can repeated operations be coalesced? DOM reads/writes separated into a read phase
then a write phase; N network calls into one; a burst of state updates into a
single commit. A loop doing one small thing per item is usually one batched thing.

### 5. Caching / memoization

Is a pure computation repeated with the same inputs? Recomputed derived state,
a selector re-run every render, a parse redone per call. Cache it — but only with
a correct invalidation story, and only if step 1 says the recompute is actually
hot. A memo on cold code is dead weight.

### 6. Worker offloading

Should this run off the main thread? CPU-bound work over ~a few ms (image/video
encode, hashing, parsing, model inference) belongs in a Web Worker / worker thread
so the main thread keeps hitting its frame or event budget.
→ For media pipelines: [references/real-time-media.md](references/real-time-media.md).
→ For DOM/paint/re-render cost: [references/web-rendering.md](references/web-rendering.md).

---

## Domain configuration

The six steps are universal; the *budgets and the named hot paths are not*. The
four references are **checklist templates** — a project fills in its own numbers
(frame budget, p99 target, payload sizes) and its own path names (which route is
hot, which pipeline is the encoder path). Adapt them on the way in the same way
the rules are adapted: a reference that still says "your streaming pipeline"
instead of the project's real one gets ignored. Keep the incident/symptom notes;
replace the placeholders.

## Output shape

Findings ranked by **hot-path impact** — the one that costs the most against its
budget first — not by how easy the fix is. For each: the impact, a one-line claim,
`file:line`, the measurement that backs it (or "unmeasured — needs a profile"),
and the concrete fix. Do not pad; an empty list is a pass.

```
PERF REVIEW — <target>   (budget: <e.g. 16ms/frame @ 60fps>)

HOT PATH — render loop, runs every frame
  [alloc] new closure + array spread per frame in onTick
          src/scene/ticker.ts:42
          measured: 6ms/frame in DevTools perf, 37% of budget
          → hoist the closure; push into a reused array instead of spread

  [block] sync JSON.parse of ~2MB frame metadata on the event loop
          src/net/frame.ts:88
          unmeasured — needs a profile; payload size confirmed ~2MB
          → parse in a worker, or stream-parse incrementally

NOT HOT
  [skip] config normaliser runs once at startup — no hot-path impact
         src/config/load.ts:12
```

Clean run:

```
PERF REVIEW — <target>
No hot-path impact. Changed code is cold / within budget.
```

## References

- [references/main-thread-patterns.md](references/main-thread-patterns.md) — event-loop / main-thread blockers and the fix for each.
- [references/memory-allocation.md](references/memory-allocation.md) — allocation-sensitive patterns and GC-pressure symptoms.
- [references/real-time-media.md](references/real-time-media.md) — camera/video/streaming frame budgets and pipeline placement.
- [references/web-rendering.md](references/web-rendering.md) — DOM/layout/paint/re-render cost.
