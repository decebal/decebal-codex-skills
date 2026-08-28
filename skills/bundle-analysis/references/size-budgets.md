# Size budgets

A budget is a byte ceiling per entry point plus a total, enforced by a gate that fails
when the built artifact exceeds it. It turns "the bundle feels big" into a number that
blocks a merge.

## Why gzip/brotli is the number that matters

A user downloads the **compressed** transfer, not the raw file — the CDN serves gzip or
brotli. So the budget is denominated in gzip (or brotli) bytes, never raw:

- Raw size overstates cost by ~3–4× for text; a raw budget punishes readable code that
  compresses fine and lets a poorly-compressing blob through.
- **Brotli** is ~15–20% smaller than gzip and is what most CDNs prefer today; gzip is the
  universal floor. Budget against whichever your CDN actually serves, and if unsure,
  budget gzip (the conservative, always-available number).

For a script that loads on third-party sites, this compounds: a **2 KB gzip** regression
across, say, 50M loads/day is ~100 GB/day of extra transfer and a slower first paint on
every one of them. A regression that is a rounding error in one load is a real cost at
fleet scale — which is exactly why it earns a gate.

## Choosing budgets

- **Per entry point**, not one global number — a regression in a rarely-loaded worker
  should not be masked by headroom in the main bundle.
- **Total gzip** across all entries as a backstop, so work that shuffles bytes between
  chunks without shrinking anything still trips.
- **Set the ceiling just above today's measured size** (e.g. current + ~2–3%), then
  ratchet it **down** as you trim. A budget far above reality enforces nothing. This is
  the ceiling-not-ban pattern: every new regression fails, without demanding a cleanup
  first.
- Budget the **entry/initial** payload separately from lazy-loaded chunks — moving code
  behind a dynamic `import()` should *pass* (it left the initial load), not fail on total.

```jsonc
// bundle-budgets.json — all values gzip bytes
{
  "entrypoints": {
    "index.js":     43008,   // 42 KB
    "worker.js":    12288,   // 12 KB
    "polyfills.js":  9216    //  9 KB
  },
  "totalGzip": 65536         // 64 KB backstop
}
```

## Enforcing in CI

The gate measures the built files and compares to the budget — it does **not** rebuild,
so it stays well under the 5-minute ceiling
([../../rules/timeouts.md](../../../rules/timeouts.md)). Run it in the pre-push hook *and*
CI, and trust the exit code over any printed summary
([../../rules/evidence-discipline.md](../../../rules/evidence-discipline.md)).

```ts
// check-budgets.ts — run: bun run check-budgets.ts ./dist
const dist = process.argv[2] ?? "./dist";
const budget = await Bun.file("bundle-budgets.json").json();

let total = 0;
let failed = false;
const fmt = (n: number) => `${(n / 1024).toFixed(1)} KB`;

for (const [name, limit] of Object.entries(budget.entrypoints) as [string, number][]) {
  const bytes = Bun.gzipSync(await Bun.file(`${dist}/${name}`).bytes(), { level: 9 }).length;
  total += bytes;
  const over = bytes > limit;
  failed ||= over;
  console.log(`${over ? "FAIL" : "ok  "} ${name}: ${fmt(bytes)} / ${fmt(limit)}`);
}

if (total > budget.totalGzip) {
  failed = true;
  console.log(`FAIL total: ${fmt(total)} / ${fmt(budget.totalGzip)}`);
}

process.exit(failed ? 1 : 0);
```

`Bun.gzipSync` is a real Bun built-in; `{ level: 9 }` pins compression so the measured
number is stable run to run. For brotli, swap in a brotli encoder and budget against
brotli bytes to match a brotli-serving CDN.

## Wire it into the diff, not just an absolute cap

An absolute budget catches "too big"; a **delta** check catches "crept up". Run the gate
on both branches (SKILL step 2) and fail on a per-entry increase over a threshold (e.g.
+1 KB gzip) even when still under budget — that is what stops a bundle from ratcheting
*up* one innocent KB per PR until it blows the ceiling all at once.
