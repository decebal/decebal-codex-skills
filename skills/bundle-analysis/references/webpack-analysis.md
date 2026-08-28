# webpack-bundle-analyzer & source-map-explorer

Two tools, two questions. `webpack-bundle-analyzer` answers "what is in *this webpack
build*" from webpack's own stats. `source-map-explorer` answers "what is in *this output
file*" from its sourcemap, and works for any bundler (Bun, esbuild, Rollup, Vite, webpack).

## webpack-bundle-analyzer

Needs a webpack stats JSON. Generate it, then open the treemap.

```bash
# 1. emit stats from the build
bunx webpack --profile --json > stats.json
#    (or in webpack.config.js: add BundleAnalyzerPlugin, or `stats: 'normal'`)

# 2. static HTML report (no server), or interactive
bunx webpack-bundle-analyzer stats.json dist/ --mode static --report report.html
bunx webpack-bundle-analyzer stats.json dist/            # opens on :8888
```

Passing the `dist/` bundle directory as the second arg lets it read the real emitted
files, which is what makes the **gzip** column accurate rather than estimated.

### Reading the treemap — three sizes, never conflate them

The mode toggle (top-left) switches which size every rectangle represents:

| Size | What it is | Use it to |
|------|-----------|-----------|
| **stat** | size reported by webpack *before* any minification | see the raw source weight of a module |
| **parsed** | size *after* webpack's minifier ran on the output | see what actually ships, unminified-gzip |
| **gzip** | the parsed output after gzip | judge real transfer cost — **the number that matters** |

A module that is huge in **stat** but tiny in **parsed** minified well. A module large in
**gzip** is your actual payload. Always end on gzip.

### What the treemap tells you

- **Rectangle area = bytes.** The biggest boxes are where the weight is; start there.
- **A `node_modules` box larger than your own `src`** means a dependency dominates —
  candidate for a lighter alternative or a dynamic import.
- **The same library appearing in two chunks** means it is duplicated — a shared chunk /
  `splitChunks` fix, or a version dedupe (two majors resolve separately).
- **A "moment/locale" or "lodash" block far bigger than expected** is the classic
  whole-library import that defeats tree-shaking (see tree-shaking-patterns.md).

## @next/bundle-analyzer

Next.js wrapper around the same treemap. Wire it in `next.config.js`:

```js
const withBundleAnalyzer = require("@next/bundle-analyzer")({ enabled: process.env.ANALYZE === "true" });
module.exports = withBundleAnalyzer({ /* next config */ });
```

```bash
ANALYZE=true bun run build   # opens client + server treemaps
```

Read the **client** bundles for what users download; the server bundle is not shipped to
the browser.

## source-map-explorer

Bundler-agnostic. It needs the output file **and** its sourcemap (inline or a sibling
`.js.map`). Build with sourcemaps on (`bun build --sourcemap`, or the bundler's flag).

```bash
bunx source-map-explorer dist/index.js                 # auto-finds dist/index.js.map
bunx source-map-explorer dist/index.js dist/index.js.map
bunx source-map-explorer dist/index.js --html sme.html # visual treemap
bunx source-map-explorer dist/index.js --json          # attribution as JSON, for a diff
bunx source-map-explorer 'dist/*.js'                    # every entry at once
```

Every byte of the bundle is attributed back to a source path, so this is how you build the
**"Top Contributors to Delta"** list: run it on both branches, subtract per source path.

### Caveats

- **No sourcemap, no attribution.** If the map is missing or wrong, bytes land in an
  `[unmapped]` bucket — a large `[unmapped]` means the map is stale, not that the code is
  mysterious. Regenerate the build.
- **Sizes are of the *mapped output*.** For transfer cost, still gzip the file (SKILL step
  1); source-map-explorer reports parsed bytes, not gzip, unless you post-process.
- Trust the file on disk over any tool's headline number — see
  [../../rules/evidence-discipline.md](../../../rules/evidence-discipline.md).
