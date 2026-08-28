# Tree-shaking patterns

Tree-shaking = the bundler drops exports nothing imports. It only works when the bundler
can *prove* an export is unused and side-effect-free. Every pattern below removes that
proof, so dead code ships anyway.

## 1. Barrel files

A barrel is an `index.ts` that re-exports a directory:

```ts
// src/utils/index.ts
export * from "./date";
export * from "./crypto";
export * from "./dom";
```

Importing one symbol from the barrel makes the bundler evaluate the whole barrel, and if
any re-exported module has side effects (or the bundler can't prove it doesn't), all of
them are retained:

```ts
import { formatDate } from "../utils";        // ← may pull crypto + dom too
import { formatDate } from "../utils/date";   // ✅ import the leaf directly
```

Detect:

```bash
grep -rnE "from ['\"][.]{1,2}(/.*)?/index['\"]" src/
grep -rnE "from ['\"][.]{1,2}/(utils|components|lib)['\"]" src/   # dir = its index
```

The cost is real: barrel imports have been measured at hundreds of ms of extra import
cost and 30–50% larger bundles. Import from the concrete module path.

## 2. The `"sideEffects"` field

In `package.json`, this tells the bundler whether files can be dropped when their exports
are unused:

```jsonc
{ "sideEffects": false }                              // every module is pure — droppable
{ "sideEffects": ["*.css", "./src/register.ts"] }     // only these have effects
```

- **Missing / `true`** → the bundler assumes every module might have a side effect and
  keeps them all. Safe, but bloated.
- **`false` when it's actually wrong** → the bundler drops a module whose evaluation was
  load-bearing (a global mutation, a polyfill, a custom-element `register()`, `import
  "./styles.css"`). The bundle shrinks and the runtime **breaks** — the worst failure,
  because it only shows up at run time on the code path that needed the effect.

Rule: set `false` only if every module is genuinely pure; otherwise enumerate the
effectful ones. CSS-only imports (`import "./x.css"`) are side effects — list the glob.

## 3. Import styles that defeat tree-shaking

```ts
import _ from "lodash";              // ❌ whole library, CJS, no shaking → ~70KB
import { debounce } from "lodash";   // ❌ still the whole CJS library
import debounce from "lodash/debounce"; // ✅ single module
import { debounce } from "lodash-es";   // ✅ ESM build, shakeable

import * as utils from "./utils";   // ❌ namespace import can retain everything
import { one } from "./utils";      // ✅ named, shakeable

const mod = require("./thing");     // ❌ CJS is not statically analyzable
```

- **Namespace imports** (`import * as X`) defeat shaking when `X` is passed around or
  indexed dynamically — the bundler can't tell which members are live.
- **Re-assigning or spreading** an imported binding forces retention.

## 4. ESM vs CJS

Tree-shaking requires **static** `import`/`export`. CommonJS (`require`, `module.exports`)
is dynamic — the bundler cannot know at build time what is used, so it keeps the whole
module. Prefer dependencies that ship an ESM build (`"module"` or `"exports"` with an
`import` condition in their `package.json`). A dep that is CJS-only will not shake no
matter how you import it.

## 5. Verify a dependency actually tree-shakes

Don't assume — prove it against the built artifact:

1. Build a throwaway entry that imports **one** symbol from the dep.
2. Measure gzip (SKILL step 1).
3. If the size is close to the dep's whole-library gzip (bundlephobia), it did **not**
   shake — it's CJS-only, side-effect-flagged, or you hit a barrel.

```bash
echo 'import { debounce } from "lodash-es"; console.log(debounce);' > /tmp/probe.ts
bun build /tmp/probe.ts --outfile /tmp/probe.js --minify
gzip -9 -c /tmp/probe.js | wc -c        # a few KB = shook; ~25KB = did not
```

Also confirm with `bunx knip` that you have no unused exports feeding dead weight into the
graph. Removing an unused package entirely beats shaking it —
[../../rules/dependency-hygiene.md](../../../rules/dependency-hygiene.md).
