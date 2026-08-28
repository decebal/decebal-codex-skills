# Allocation & GC pressure

On a hot path the enemy is usually not a slow line — it is **allocation rate**.
Every object, array, and closure you create per frame or per request is garbage
the collector must later trace and free, and a GC pause lands as a dropped frame
or a latency spike at a moment you did not choose. The fix is almost always
"allocate less on this path", not "allocate faster".

**Template** — the patterns are general; the *frequency* that makes each one
matter is yours (per frame at 60fps? per request at 10k rps?). An allocation that
is fine once at startup is a leak-shaped cost 60 times a second.

| # | Pattern | Detect | Cost | Fix |
|---|---|---|---|---|
| A1 | Object/array literal churn per iteration | `rg -n '\{.*\}|\[.*\]'` inside a per-frame / per-request body; look for `.map`/`.filter`/`.slice` chains that each allocate | N objects per pass → N garbage per pass; chained array methods each allocate a new array | Reuse a preallocated object/array; do a single `for`/`for-of` pass instead of chained `.map().filter()`; mutate in place where ownership allows |
| A2 | Closures created in a hot loop | `rg -n '=>|function'` inside a loop or a per-frame callback; callbacks passed to `.forEach`/`.map` each iteration | Each closure is an allocation and captures its environment, keeping it alive | Hoist the function out of the loop; pass a stable reference; for React see [web-rendering.md](web-rendering.md) (`useCallback`) |
| A3 | Array spread vs push | `rg -n '\.\.\.'` in a loop or accumulation (`acc = [...acc, x]`, `arr.concat` in a fold) | `[...acc, x]` copies the whole accumulator every step → O(n²) allocation | `arr.push(x)` (O(1) amortized, in place); spread once at the end if you need immutability |
| A4 | String building by concatenation | `rg -n '\+=\s*["\x60'\'']'` in a loop; `str = str + chunk` accumulation | Strings are immutable; `+=` in a loop reallocates the growing string each step → O(n²) | Push chunks to an array and `join('')` once; or a streaming writer; template a whole line at a time, not char by char |
| A5 | No buffer reuse / pooling | new `Buffer.alloc` / `new Uint8Array` / new canvas / new `ArrayBuffer` per frame or per message | Large typed-array allocations per frame are the classic GC-sawtooth source | Allocate once and reuse; a small free-list pool for fixed-size buffers; `subarray`/`set` into a reused buffer instead of a fresh one |
| A6 | Boxing / megamorphic shapes | objects built with varying key sets on a hot path; mixing `number` and `object` in one hot array | Deopt: the JIT cannot keep one hidden class; also more allocation | Build objects with a consistent key order/shape; keep hot arrays monomorphic (all same type) |

## GC-pressure symptoms (how you know this is the problem)

- **Sawtooth heap:** memory climbs steadily then drops sharply, repeatedly. The
  drops are GC. Read it in DevTools Memory → allocation timeline, or Node
  `--trace-gc`. Steep sawtooth on a hot path = allocation rate too high.
- **Frame drops that correlate with GC:** in the Performance panel, dropped frames
  line up with GC (yellow) events. That is the tell that allocation, not compute,
  is the cause.
- **Rising baseline (a real leak, different bug):** the heap floor climbs and never
  returns — retained references, not churn. Take two heap snapshots and diff
  retained size; look for detached DOM nodes, un-removed listeners, growing caches.
  This is a correctness leak, not GC pressure; both show up here, fix them
  differently.

## Confirm, don't guess

Measure allocation with the allocation profiler (DevTools "Allocation
instrumentation on timeline", or `--prof` / heap snapshots in Node), not by
reading the code. The line that *looks* expensive is often cold; the real churn
is usually a chained array method or a per-frame closure you skimmed past.
