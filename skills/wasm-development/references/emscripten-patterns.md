# Emscripten patterns (C/C++ → WASM)

Emscripten's `emcc` (C) / `em++` (C++) compile through Clang/LLVM and emit a
`.wasm` plus a JS loader. Install and activate the SDK first:

```bash
git clone https://github.com/emscripten-core/emsdk.git
emsdk/emsdk install latest
emsdk/emsdk activate latest
source emsdk/emsdk_env.sh   # puts emcc/em++ on PATH
emcc -v                     # confirm
```

## Output form and module shape

The output extension controls the shape:

```bash
emcc a.c -o out.js      # a.wasm + a UMD-ish out.js loader
emcc a.c -o out.mjs     # ES module loader (implies MODULARIZE)
emcc a.c -o out.html    # + an HTML shell that loads it (demo/testing)
emcc a.c -o out.wasm    # standalone wasm, no JS glue (freestanding/WASI-ish)
```

For app integration use `MODULARIZE` so the output is a factory returning a
`Promise`, not code that runs on load, plus `EXPORT_ES6` for a real ES module:

```bash
em++ src/*.cpp -o build/mymod.mjs \
  -sMODULARIZE=1 \
  -sEXPORT_ES6=1 \
  -sEXPORT_NAME=createMyModule \
  -sENVIRONMENT=web,worker \
  -sALLOW_MEMORY_GROWTH=1
```

```js
import createMyModule from "./build/mymod.mjs";
const Module = await createMyModule();   // instantiated, ready
```

- `EXPORT_ES6=1` **requires** `MODULARIZE=1`.
- `ENVIRONMENT=web,worker` drops Node-only code paths from the glue (smaller).

## Memory flags

- `-sALLOW_MEMORY_GROWTH=1` — let linear memory grow at runtime instead of a hard
  cap. **Cost:** JS-side `HEAP*` views are re-created on every growth, so never
  cache a view across a call that might allocate (see js-wasm-integration.md).
- `-sINITIAL_MEMORY=64MB` — starting heap size.
- `-sMAXIMUM_MEMORY=512MB` — ceiling when growth is on.
- `-sSTACK_SIZE=1MB` — the linear-memory stack (deep recursion / large locals).

## Exporting functions: raw exports vs Embind

**Raw exports + ccall/cwrap** — lightest, for a C ABI. Export the native symbols
(note the leading underscore) and the runtime helpers you call:

```bash
emcc math.c -o math.mjs -sMODULARIZE=1 -sEXPORT_ES6=1 \
  -sEXPORTED_FUNCTIONS=_add,_malloc,_free \
  -sEXPORTED_RUNTIME_METHODS=ccall,cwrap,getValue,setValue
```

```js
const Module = await createMyModule();
// one-shot call
const sum = Module.ccall("add", "number", ["number", "number"], [2, 3]);
// reusable wrapper
const add = Module.cwrap("add", "number", ["number", "number"]);
add(2, 3);
```

Keep functions alive across dead-code elimination from C with
`EMSCRIPTEN_KEEPALIVE` (in `<emscripten/emscripten.h>`).

**Embind** — for C++ classes, overloads, `std::string`/`std::vector` binding.
Compile with `-lembind` (modern; `--bind` is the older spelling) and declare
bindings:

```cpp
#include <emscripten/bind.h>
using namespace emscripten;

struct Point { float x, y; };
float dist(const Point& p) { return std::sqrt(p.x*p.x + p.y*p.y); }

EMSCRIPTEN_BINDINGS(geo) {
  value_object<Point>("Point").field("x", &Point::x).field("y", &Point::y);
  function("dist", &dist);
}
```

```bash
em++ geo.cpp -lembind -o geo.mjs -sMODULARIZE=1 -sEXPORT_ES6=1
```

```js
const Module = await createMyModule();
Module.dist({ x: 3, y: 4 }); // 5
```

## Filesystem

Emscripten emulates a POSIX FS in JS. Backends:

- **MEMFS** — default, in-memory, gone on reload. Files you `--preload-file` or
  `--embed-file` at build time land here.
- **NODEFS** — mounts the real host FS; **Node only**. Link with `-lnodefs.js`,
  then `FS.mount(NODEFS, { root: "." }, "/host")`.
- **IDBFS** — persists to IndexedDB in the browser. Link with `-lidbfs.js`, mount,
  and call `FS.syncfs(false, cb)` to flush.

Bundle assets into MEMFS at build time:

```bash
emcc app.c -o app.mjs --preload-file assets   # → app.data, fetched at init
emcc app.c -o app.mjs --embed-file config.json  # embedded in the wasm/js (small files)
```

Force the FS glue in even when the linker thinks it's unused: `-sFORCE_FILESYSTEM=1`.

## Threads (pthreads) and the SharedArrayBuffer requirement

WASM threads use `SharedArrayBuffer`, which browsers gate behind
**cross-origin isolation**. Two things are required:

1. Build with pthreads:
   ```bash
   em++ par.cpp -o par.mjs -pthread -sPTHREAD_POOL_SIZE=8 \
     -sMODULARIZE=1 -sEXPORT_ES6=1
   ```
   `-pthread` is needed at **both** compile and link. `-sPROXY_TO_PTHREAD` runs
   `main()` off the browser main thread so it can block.
2. Serve the page with COOP + COEP so `SharedArrayBuffer` is enabled:
   ```
   Cross-Origin-Opener-Policy: same-origin
   Cross-Origin-Embedder-Policy: require-corp
   ```
   Without both headers, `SharedArrayBuffer` is undefined and the module fails to
   start. Cross-origin subresources then also need CORP/CORS.

## Optimization flags (build side)

- `-O0` (default, debug) … `-O1` / `-O2` / `-O3` (speed) — `-O2`/`-O3` for release.
- `-Os` / `-Oz` — optimize for **size** (`-Oz` most aggressive); matters for a
  binary downloaded on every load.
- `-flto` — link-time optimization; pass at compile **and** link.
- `-g` levels for debug info; `--profiling-funcs` keeps function names for
  profiling without full debug info. See debugging.md and optimization.md.

## CMake projects

Drive CMake through the Emscripten wrappers — they inject the toolchain file:

```bash
emcmake cmake -S . -B build-wasm -DCMAKE_BUILD_TYPE=Release
emmake make -C build-wasm -j
```

`emcmake` sets `CMAKE_TOOLCHAIN_FILE` to Emscripten's
`cmake/Modules/Platform/Emscripten.cmake` for you. Put link-time `-s` flags in
`target_link_options`, and to emit the JS/HTML shell set the suffix:

```cmake
set(CMAKE_EXECUTABLE_SUFFIX ".mjs")
target_link_options(mymod PRIVATE
  "-sMODULARIZE=1" "-sEXPORT_ES6=1" "-sALLOW_MEMORY_GROWTH=1")
```

## See also

- Marshalling data / calling from JS: [js-wasm-integration.md](js-wasm-integration.md)
- Shrinking the output: [optimization.md](optimization.md)
- CSP / COOP-COEP security review: [../../security-review/SKILL.md](../../security-review/SKILL.md)
