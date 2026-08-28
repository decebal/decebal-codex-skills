# WASM optimization

A `.wasm` is downloaded, then compiled, on every cold load. Shrink it and prove
the shrink with numbers.

**Measure, do not guess.** Record byte sizes before and after every change; a flag
that "should" help sometimes regresses. See
[../../../rules/evidence-discipline.md](../../../rules/evidence-discipline.md) —
read the artifact, not a tool's summary line.

## wasm-opt (binaryen) — the main lever

`wasm-opt` from [binaryen](https://github.com/WebAssembly/binaryen) rewrites the
module: DCE, inlining, local coalescing, size reductions.

```bash
# brew install binaryen  (or download a binaryen release)
wasm-opt -O3 in.wasm -o out.wasm      # optimize for speed
wasm-opt -Oz in.wasm -o out.wasm      # optimize hardest for size
```

Levels: `-O1`/`-O2`/`-O3`/`-O4` (increasing speed effort), `-Os` (size, some
speed), `-Oz` (size above all). For a web module `-Oz` or `-Os` usually wins,
`-O3`/`-O4` when compute dominates. Try both and compare bytes and a benchmark.

Toolchain integration:

- **Rust/wasm-pack** runs `wasm-opt` automatically on `--release`. Tune or disable
  it in `Cargo.toml`:
  ```toml
  [package.metadata.wasm-pack.profile.release]
  wasm-opt = ["-Oz"]      # or  wasm-opt = false  to skip it
  ```
- **Emscripten** runs binaryen passes as part of `-O2`/`-O3`/`-Oz` at link time;
  running `wasm-opt` again afterwards can still find more.

## Dead code

Ship only reachable code:

- **Rust**: `lto = true`, `codegen-units = 1`, `opt-level = "z"/"s"`, `strip =
  true` in `[profile.release]`; enable only the `web-sys` features you use.
- **Emscripten**: `-flto`; export only what you call
  (`-sEXPORTED_FUNCTIONS`), and prefer `-Oz` for size. `EMSCRIPTEN_KEEPALIVE` or
  the exported-functions list is what keeps a symbol from being stripped — keep
  the list minimal.

## Strip debug/symbol sections

Debug info and names bloat a release binary. Strip after building (keep an
unstripped copy for symbolication):

```bash
wasm-opt -Oz --strip-debug --strip-producers in.wasm -o out.wasm
# or, from wabt:  wasm-strip out.wasm
```

Rust: `strip = true` in the release profile does this at build time.

## Profile the bytes: twiggy (Rust) / bloaty / disassembly

Find *what* is big before cutting:

```bash
cargo install twiggy
twiggy top   pkg/mymod_bg.wasm     # largest items by size
twiggy dominators pkg/mymod_bg.wasm # what each item retains (why it's there)
twiggy monos pkg/mymod_bg.wasm     # monomorphization bloat (generic blowup)
```

`bloaty mymod.wasm` also breaks down sections for any toolchain. Big single items
are often a formatting/panic machinery pull-in (`core::fmt`), a heavy generic, or
an accidental `std` dependency.

## SIMD — opt-in, and must be feature-detected

WASM SIMD (128-bit) is widely supported but **not universal**; a module built with
SIMD instructions fails to instantiate on an engine without it. Build a SIMD
variant only if you'll gate it:

- **Emscripten**: `-msimd128` (plus `-O3`). Use `<wasm_simd128.h>` intrinsics or
  let autovectorization use it.
- **Rust**: `RUSTFLAGS="-C target-feature=+simd128" wasm-pack build ...`, and the
  `core::arch::wasm32` SIMD intrinsics.

Detect support at runtime and load the right build. The `wasm-feature-detect`
package (GoogleChromeLabs) probes safely:

```js
import { simd, threads } from "wasm-feature-detect";
const url = (await simd()) ? "/mymod.simd.wasm" : "/mymod.wasm";
```

Ship two artifacts (SIMD + scalar) and pick per client, or the scalar build alone.

## How to measure

Compare the numbers that a CDN actually serves — raw is misleading, gzip/brotli is
the wire cost:

```bash
wc -c < out.wasm               # raw bytes
gzip -9 -c out.wasm | wc -c    # gzip bytes
brotli -q 11 -c out.wasm | wc -c   # brotli bytes (brew install brotli)
```

Track these per build and gate regressions with the sibling
[../../bundle-analysis/SKILL.md](../../bundle-analysis/SKILL.md), which also
attributes JS-side bytes and enforces a budget in CI. Pair a byte number with a
runtime benchmark so a size win that halves throughput is caught.

## See also

- Build flags that feed these tools: [emscripten-patterns.md](emscripten-patterns.md), [rust-wasm-patterns.md](rust-wasm-patterns.md)
