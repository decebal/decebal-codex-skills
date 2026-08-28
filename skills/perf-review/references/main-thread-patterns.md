# Main-thread / event-loop blockers

The main thread (browser) and the event loop (Node) run one thing at a time.
Anything synchronous and slow on them freezes input, animation, and every pending
request until it returns. The symptom is a stall, not a slow line — a 120 ms sync
call drops ~7 frames at 60fps and stalls every concurrent request for 120 ms.

**Template** — fill in your own thresholds and the names of your hot handlers.
The default budget: nothing synchronous over ~a few ms on a path that runs per
frame or per request. Confirm with a profile (DevTools Performance, `--prof`, a
`console.time`/`performance.now` span) before and after — do not eyeball it.

| # | Pattern | Detect | Why it blocks | Fix |
|---|---|---|---|---|
| B1 | Sync parse of a large payload | `rg -n 'JSON\.parse\('` on request/message handlers; check the payload size | Parsing a multi-MB string is O(size) synchronous CPU on the thread that received it | Parse in a worker; or stream-parse incrementally (e.g. a SAX-style / chunked parser); or push the shape server-side so the client parses less |
| B2 | Sync crypto / hashing | `rg -n 'Sync\(|pbkdf2Sync|scryptSync|createHash\([^)]*\)\.update\(.*\)\.digest|randomBytesSync'` | Key derivation and per-item hashing in a loop are pure CPU and can run tens to hundreds of ms | Use async WebCrypto (`crypto.subtle.*`) or the async Node crypto callbacks; move a hashing loop to a worker; batch (see real-time-media B/crypto) |
| B3 | Layout thrashing (read-after-write) | `rg -n 'offsetWidth|offsetHeight|getBoundingClientRect|getComputedStyle|scrollTop|clientHeight'` interleaved with style/DOM writes in the same loop | Reading a layout property after a write forces a **synchronous reflow**; interleaving R/W/R/W in a loop makes it O(n) reflows | Batch: do all reads first, then all writes. Cache the read value outside the loop. Schedule writes in `requestAnimationFrame`. See [web-rendering.md](web-rendering.md) |
| B4 | Microtask flood | `rg -n 'await'` inside a `for`/`while` over a large collection; deep recursive `.then()` chains | A tight `await`-per-item loop serializes N round-trips and can starve rendering between microtask drains | `Promise.all` for independent work; chunk the loop and yield (`await scheduler.yield()` / `setTimeout(0)` / `MessageChannel`) so paint and input can interleave |
| B5 | Long synchronous compute | profile for a single self-time span > a frame; large `sort`/`map`/`reduce` over big arrays, regex over huge strings, nested loops | One long call = one long stall; there is no preemption on the main thread | Chunk with cooperative yielding; move to a worker (see [real-time-media.md](real-time-media.md) for the encode/inference case); precompute or memoize (SKILL step 5) |
| B6 | Sync I/O on the event loop (Node) | `rg -n 'readFileSync|writeFileSync|readdirSync|existsSync|execSync'` on a request path | Blocks the single event loop for the whole file/exec duration — every concurrent request waits | Use the async (`fs/promises`) form; read once at startup and cache if the file is static |

## How to confirm, not guess

- **Before:** open a profile and find the long task / self-time span. A blocker is
  a contiguous synchronous span, not a total. If you cannot see one, the code may
  not be the bottleneck — do not "fix" it blind.
- **After:** re-profile the same scenario. The span should shrink or move off the
  main thread. A change with no measured delta is not a performance fix; drop it.
- **Long Tasks** (browser) surface directly: `PerformanceObserver` on
  `longtask`, or the DevTools Performance panel's red-cornered task blocks.
