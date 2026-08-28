---
name: bundle-analysis
description: "Measure, diff, and gate JavaScript/TypeScript bundle size. Use when the user wants to check bundle size, analyze a bundle, see a size diff, asks what's the bundle impact of a change, or wants to tree-shake a dependency. Covers raw/gzip/brotli snapshots per entry point, base-vs-branch diff reports, dependency attribution, tree-shaking validation, and CI budget enforcement. Triggers on: check bundle size, analyze bundle, size diff, bundle impact, tree-shake."
---

# Bundle Analysis

Ship a smaller bundle by measuring it, not by guessing. For a script that loads on
third-party sites, gzip/brotli bytes are multiplied across millions of loads — a 3 KB
regression is a real cost, so treat the number as a gated artifact.

**Measure the built artifact, not a proxy.** A `package.json` version bump, a lockfile
diff, or a bundler's own summary line can all disagree with the bytes on disk. Read the
file. See [../../rules/evidence-discipline.md](../../rules/evidence-discipline.md).

Work these five capabilities as a workflow. Steps 1–2 need no bundler tooling.

## 1. Size snapshot — raw + gzip + brotli per entry point

For each built entry point, capture three numbers. Gzip and brotli are what a CDN
actually serves, so they are the numbers that matter.

Zero-dependency fallback (works on any built file, no bundler plugin):

```bash
# raw bytes
wc -c < dist/index.js
# gzip bytes — pin the level so every measurement is comparable
gzip -9 -c dist/index.js | wc -c
# brotli bytes (brew install brotli; -q 11 is max, the CDN default)
brotli -q 11 -c dist/index.js | wc -c
```

Snapshot every entry point, not just the main one. Record the numbers before you touch
anything — that recorded set is the base for step 2.

## 2. Diff analysis — current branch vs base branch

1. Build and snapshot the **base** branch (usually `origin/main`) into a temp dir.
2. Build and snapshot the **current** branch.
3. Compare per entry point and render the report table below.

Report format to reproduce:

```
## Bundle Size Report

| Entry Point   | Current   | Base      | Delta      | Status |
|---------------|-----------|-----------|------------|--------|
| index.js      | 42.1 KB   | 39.8 KB   | +2.3 KB    | ⚠️     |
| worker.js     | 11.4 KB   | 11.4 KB   | 0 B        | ✅     |
| polyfills.js  | 8.0 KB    | 9.2 KB    | -1.2 KB    | ✅     |

All sizes are gzip. Status ⚠️ = over budget or regressed; ✅ = within budget.

### Top Contributors to Delta
- +2.1 KB  date-fns (newly imported in src/format.ts)
- +0.3 KB  src/report/table.ts (new module)
- -0.1 KB  tree-shaken dead export in src/utils.ts
```

Sizes in the table are **gzip** (state the unit explicitly). Derive "Top Contributors"
from step 3.

## 3. Dependency analysis — what's actually in the bundle

Find which `node_modules` dominate the bytes.

- **source-map-explorer** (any bundler that emits sourcemaps) — attributes every byte of
  the output back to its source module:
  ```bash
  bunx source-map-explorer dist/index.js
  bunx source-map-explorer dist/index.js --json > sme.json   # machine-readable
  ```
- **A dependency's standalone cost** before adding it — bundlephobia API:
  ```bash
  curl -s "https://bundlephobia.com/api/size?package=date-fns@3.6.0"
  # → { "size": <minified>, "gzip": <bytes>, "dependencyCount": N, ... }
  ```

Before adding any dependency, run this and check the blast radius — see
[../../rules/dependency-hygiene.md](../../rules/dependency-hygiene.md). A one-liner that
drags in 40 KB gzip is rarely worth it.

Deeper reading, plus webpack-specific tooling: [references/webpack-analysis.md](references/webpack-analysis.md).

## 4. Tree-shaking validation

Three defeats of tree-shaking to hunt for:

- **Barrel imports** — importing from an index re-export pulls the whole barrel unless the
  bundler can prove otherwise:
  ```bash
  grep -rnE "from ['\"][.]{1,2}(/.*)?/index['\"]" src/   # importing a barrel
  grep -rnE "from ['\"]lodash['\"]" src/                 # whole-lib import, not lodash-es/x
  ```
- **`"sideEffects"`** in `package.json`. `"sideEffects": false` lets the bundler drop
  unused modules — but it's a **lie** if any module mutates global state, registers a
  polyfill, or imports CSS purely for effect. Wrong here = broken runtime; missing here =
  bloated bundle. List the real ones: `"sideEffects": ["*.css", "./src/polyfill.ts"]`.
- **Unused exports** shipped anyway:
  ```bash
  bunx knip                    # unused exports, files, and deps
  ```

Full pitfall catalogue (ESM vs CJS, verifying a dep tree-shakes):
[references/tree-shaking-patterns.md](references/tree-shaking-patterns.md).

## 5. Budget enforcement

Define budgets per entry point plus a total-gzip ceiling, then fail CI on regression.

```jsonc
// bundle-budgets.json — gzip bytes
{
  "entrypoints": { "index.js": 43008, "worker.js": 12288 },
  "totalGzip": 61440
}
```

Gate: measure (step 1), compare to the budget, exit non-zero when any entry or the total
is over. Keep the gate under the 5-minute ceiling — it measures files, it does not
rebuild the world. Run it in the pre-push hook AND CI, and read the exit code, not a
summary line.

Budget-picking rationale and a ready-to-run enforcement script:
[references/size-budgets.md](references/size-budgets.md).

## Tool cheat sheet

| Tool | Bundler | Invoke |
|------|---------|--------|
| raw/gzip/brotli fallback | any built file | `wc -c` / `gzip -9 -c` / `brotli -q 11 -c` |
| source-map-explorer | any with sourcemaps | `bunx source-map-explorer dist/*.js` |
| webpack-bundle-analyzer | webpack | `bunx webpack-bundle-analyzer stats.json dist/` |
| @next/bundle-analyzer | Next.js | `ANALYZE=true bun run build` |
| bundlephobia API | dep lookup | `curl -s "https://bundlephobia.com/api/size?package=<name>@<ver>"` |
| knip | any | `bunx knip` |
