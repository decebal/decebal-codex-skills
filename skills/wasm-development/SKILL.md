---
name: wasm-development
description: "Build and integrate WebAssembly across both major toolchains — C/C++ via Emscripten (emcc/em++) and Rust via wasm-pack + wasm-bindgen. Covers the JS/WASM memory boundary, off-main-thread execution in Web Workers, CSP requirements, size optimization, and debugging. Use when the user wants to build WASM, compile C/C++ or Rust to WebAssembly, set up Emscripten or wasm-pack, call a WASM module from JavaScript, pass data across the JS/WASM boundary, or shrink/debug a .wasm binary. Triggers on: build WASM, Emscripten, wasm-pack, WebAssembly, wasm-bindgen, WASM module."
---

# WASM Development

Building and integrating WebAssembly. First pick the toolchain, then handle the
three concerns that bite regardless of toolchain: the memory boundary, threading,
and CSP.

## 1. Choose the toolchain

| Source language | Toolchain | Build command | Fits when |
|---|---|---|---|
| C / C++ | Emscripten (`emcc` / `em++`) | `emcc src.c -o out.mjs` | Porting existing C/C++, SDL/OpenGL apps, large native libs, when you need the Emscripten runtime (filesystem, pthreads glue) |
| Rust | wasm-pack + wasm-bindgen | `wasm-pack build --target web` | Greenfield WASM, tight JS interop with rich types, small self-contained modules, when you want generated TypeScript `.d.ts` |

Both emit a `.wasm` plus a JS loader/glue file. Do **not** hand-write the glue —
each toolchain generates it and keeps the ABI in sync.

- C/C++ deep dive: [emscripten-patterns.md](references/emscripten-patterns.md)
- Rust deep dive: [rust-wasm-patterns.md](references/rust-wasm-patterns.md)

## 2. The JS/WASM boundary is the hard part

WASM memory is a single linear `ArrayBuffer`. You **cannot** pass JS objects,
strings, or arrays directly — you pass **numbers** (i32/i64/f32/f64), which for
anything larger than a scalar means a **pointer** (an offset into that buffer).

- **Small scalars** cross for free as function args/returns.
- **Strings / structs / typed arrays** must be marshalled: allocate inside WASM
  memory (`Module._malloc` / a wasm-bindgen accessor), copy the bytes in, pass the
  pointer, and free it afterwards. Emscripten's `cwrap`/Embind and Rust's
  wasm-bindgen generate this copy for you.
- **Large data (e.g. video frames)** is either **copied** into WASM memory or
  **shared** by taking a `TypedArray` view directly over `WebAssembly.Memory.buffer`
  and having WASM write into it — no copy, but you own the lifetime.

Marshalling patterns, the detached-buffer gotcha after memory growth, and worker
message passing: [js-wasm-integration.md](references/js-wasm-integration.md).

## 3. Run heavy WASM off the main thread

CPU-bound WASM on the main thread freezes the UI. Move it to a **Web Worker**:

- Instantiate with `WebAssembly.instantiateStreaming(fetch(url), imports)` inside
  the worker (streaming compile — the server MUST serve `application/wasm`).
- Ship data in and out with `postMessage`; transfer `ArrayBuffer`s as
  **Transferables** (`worker.postMessage(buf, [buf])`) to avoid a structured-clone
  copy.

Full worker + transferable pattern: [js-wasm-integration.md](references/js-wasm-integration.md).

## 4. CSP: WASM needs an explicit allowance

Under a strict Content-Security-Policy, instantiating WebAssembly requires
`'wasm-unsafe-eval'` in `script-src` — **not** `'unsafe-eval'` (that older, broader
directive also works but grants JS `eval`, which you do not want):

```
Content-Security-Policy: script-src 'self' 'wasm-unsafe-eval';
```

Get the policy reviewed — see the sibling [../security-review/SKILL.md](../security-review/SKILL.md).
Threading (SharedArrayBuffer) adds COOP+COEP header requirements — covered in
[emscripten-patterns.md](references/emscripten-patterns.md).

## 5. Size matters — a `.wasm` is downloaded and compiled on every load

Always run the binary through `wasm-opt` and strip it before shipping. Measure the
gzip/brotli bytes, do not guess — read the artifact, not a tool's summary line
([../../rules/evidence-discipline.md](../../rules/evidence-discipline.md)).
Optimization levels, SIMD, and how to measure:
[optimization.md](references/optimization.md). Attribute the bytes and gate the
size with the sibling [../bundle-analysis/SKILL.md](../bundle-analysis/SKILL.md).

## 6. When it breaks

`memory access out of bounds`, `unreachable executed`, and function-signature
traps have specific causes. Source-level stepping needs DWARF (Chrome DevTools
C/C++ extension) or source maps: [debugging.md](references/debugging.md).

## References

- [emscripten-patterns.md](references/emscripten-patterns.md) — emcc/em++ and CMake flags, Embind vs ccall/cwrap, filesystems, pthreads.
- [rust-wasm-patterns.md](references/rust-wasm-patterns.md) — wasm-pack targets, `#[wasm_bindgen]`, JsValue, serde, closures, web-sys/js-sys.
- [js-wasm-integration.md](references/js-wasm-integration.md) — loading, memory sharing, workers, error handling across the boundary.
- [optimization.md](references/optimization.md) — wasm-opt, dead code, SIMD, and measuring.
- [debugging.md](references/debugging.md) — DWARF/source maps, common RuntimeErrors, profiling.
