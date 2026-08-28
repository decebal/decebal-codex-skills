# WASM debugging

WASM traps give terse errors and, by default, no source lines. Get a signal
first, then read it — do not theorize from a bare `RuntimeError`. See
[../../../rules/debugging-discipline.md](../../../rules/debugging-discipline.md):
instrument before guessing, and revert a failed fix before layering the next.

## Step into source: DWARF and source maps

Ship debug info in a dev build so DevTools maps wasm back to C/C++/Rust source:

- **Emscripten**
  - `-g` embeds DWARF for source-level stepping.
  - `-gseparate-dwarf=out.debug.wasm` keeps the DWARF in a side file (production
    binary stays small, symbols still available).
  - `-gsource-map` emits a `.wasm.map` instead, for engines/tools that want source
    maps over DWARF.
- **Rust/wasm-pack**: `wasm-pack build --dev` (or `--profiling`) keeps debug info;
  a release build strips it. `[profile.dev] debug = true` is the default.

Then, in **Chrome DevTools**, install the **"C/C++ DevTools Support (DWARF)"**
extension. With it and a `-g` build you get original source files in the Sources
panel, breakpoints on source lines, and variable inspection — instead of raw
`wasm-function[123]` frames.

Always keep an **unstripped** copy of the release binary; you cannot symbolicate a
stack trace from a production `.wasm` without it.

## Common failure modes

| Error (JS-side `RuntimeError`) | Usual cause | First checks |
|---|---|---|
| `memory access out of bounds` | pointer past the heap, use-after-free, wrong length in a copy, a **detached buffer** after growth | re-create typed-array views after any allocating call (js-wasm-integration.md); check malloc/free pairing; Emscripten `-sSAFE_HEAP=1` |
| `unreachable executed` | a Rust `panic!`/`unwrap` on `None`/`Err`, a C++ `abort()`, or an assertion — all lower to the `unreachable` opcode | Rust: `console_error_panic_hook::set_once()` for a real stack; Emscripten: `-sASSERTIONS=1` |
| `call_indirect to a null table entry` / `function signature mismatch` | calling a freed/leaked function pointer (a dropped Rust `Closure` — you forgot `.forget()` or to keep it alive), or an ABI mismatch | keep the `Closure` alive (rust-wasm-patterns.md); confirm arg/return types match the binding |
| `table index is out of bounds` | dynamic call through a stale/invalid function index | check function-pointer lifetimes and dynamic dispatch |
| `Incorrect response MIME type` (at load) | server not serving `application/wasm` for `instantiateStreaming` | fix the MIME type or use the arrayBuffer fallback (js-wasm-integration.md) |
| `Maximum call stack` / stack overflow inside wasm | deep recursion / large stack locals overflowing the linear-memory stack | Emscripten `-sSTACK_SIZE=`, `-sSTACK_OVERFLOW_CHECK=2` |

## Turn on the runtime guards (dev only — they cost speed/size)

Emscripten:

- `-sASSERTIONS=1` — runtime sanity checks with readable messages.
- `-sSAFE_HEAP=1` — traps on out-of-bounds / misaligned memory access at the exact
  site instead of corrupting silently.
- `-sSTACK_OVERFLOW_CHECK=2` — detects stack overflow with a named error.
- `-fsanitize=address` / `-fsanitize=undefined` — ASan/UBSan work under
  Emscripten and pinpoint the offending line.

Rust: `console_error_panic_hook` for panic stacks; build `--dev` for `debug_assert!`
and overflow checks; `web_sys::console::log_*` / the `log` crate with
`console_log` for print-debugging across the boundary.

## Profiling

- **Chrome DevTools → Performance**: records wasm frames inline with JS. Build with
  names so frames are readable — Emscripten `--profiling-funcs` (keeps function
  names without full debug info) or `-g2`; a fully stripped release shows only
  `wasm-function[n]`.
- Find hot functions, then confirm a fix moved the number — measure, don't assume
  (see the evidence rule linked in [optimization.md](optimization.md)).
- For *size* (a different axis from speed), profile the binary with `twiggy` /
  `bloaty` — see [optimization.md](optimization.md).

## See also

- The detached-buffer and lifetime bugs behind most traps: [js-wasm-integration.md](js-wasm-integration.md)
- Closure lifetime rules (null-table-entry traps): [rust-wasm-patterns.md](rust-wasm-patterns.md)
