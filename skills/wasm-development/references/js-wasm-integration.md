# JS ↔ WASM integration

Loading, marshalling, workers, and error handling — the same rules whether the
`.wasm` came from Emscripten or Rust.

## Loading: instantiateStreaming with a MIME fallback

`WebAssembly.instantiateStreaming` compiles the wasm **while it downloads** — the
fastest path. It requires the server to send `Content-Type: application/wasm`; if
it doesn't, the call throws `Incorrect response MIME type`. Always keep a
non-streaming fallback:

```js
async function loadWasm(url, imports = {}) {
  try {
    return await WebAssembly.instantiateStreaming(fetch(url), imports);
  } catch (e) {
    // misconfigured MIME type, or a browser without streaming support
    const bytes = await (await fetch(url)).arrayBuffer();
    return await WebAssembly.instantiate(bytes, imports);
  }
}

const { instance, module } = await loadWasm("/mymod.wasm");
instance.exports.add(2, 3);
```

The toolchain-generated loaders (Emscripten's `createModule()`, wasm-pack's
`init()`) already do this — use them for anything with glue. Hand-instantiate only
for a bare, glue-free `.wasm`.

## The memory boundary in practice

WASM linear memory is a `WebAssembly.Memory` whose `.buffer` is an `ArrayBuffer`.
Read/write it through a typed-array **view**:

```js
const mem = instance.exports.memory;      // WebAssembly.Memory
const heap = new Uint8Array(mem.buffer);  // a view, not a copy
```

Pass a JS array **into** WASM: allocate inside wasm memory, copy, call, free.

```js
function withBytes(instance, data /* Uint8Array */, fn) {
  const { malloc, free } = instance.exports;
  const ptr = malloc(data.length);
  new Uint8Array(instance.exports.memory.buffer, ptr, data.length).set(data);
  try { return fn(ptr, data.length); }
  finally { free(ptr); }
}
```

Read data **out**: the function returns a pointer (offset); slice a view at that
offset. Copy it out (`.slice()`) if you need it to survive the next WASM call.

## The detached-buffer gotcha (read this before caching any view)

When memory grows — `memory.grow()`, or any WASM call that allocates under
`ALLOW_MEMORY_GROWTH` / a growable Rust heap — the **old `ArrayBuffer` is
detached** (its `byteLength` becomes 0) and a new one is allocated. Every typed
array you made over the old buffer now points at nothing.

```js
// BUG: `heap` may be detached after the call that grew memory
const heap = new Uint8Array(instance.exports.memory.buffer);
instance.exports.process_and_maybe_grow();
heap[0];  // ⛔ reads a detached (zero-length) buffer — 0 / undefined / throws
```

Rule: **re-create the view from `memory.buffer` after any call that can
allocate.** Never store a long-lived typed array across boundary calls.
Emscripten reassigns `Module.HEAP8`/`HEAPU8`/`HEAPF32` on growth for the same
reason — read `Module.HEAPU8` fresh each time, don't alias it into a local.

## Off-main-thread: Web Worker + Transferables

Run CPU-bound WASM in a worker so the UI thread stays responsive. Instantiate the
module **inside** the worker, then move buffers across with `postMessage`,
transferring the `ArrayBuffer` so it isn't structure-cloned (copied):

```js
// main.js
const worker = new Worker(new URL("./worker.js", import.meta.url), { type: "module" });
const frame = new Uint8Array(width * height * 4);   // e.g. an RGBA video frame
// transfer the underlying buffer — ownership moves, no copy; `frame` is now empty here
worker.postMessage({ buf: frame.buffer, width, height }, [frame.buffer]);
worker.onmessage = (e) => useResult(new Uint8Array(e.data.buf));
```

```js
// worker.js
import init, { process_frame } from "./pkg/mymod.js";
const ready = init();                        // instantiate once, reuse
onmessage = async ({ data: { buf, width, height } }) => {
  await ready;
  const out = process_frame(new Uint8Array(buf), width, height); // Uint8Array
  postMessage({ buf: out.buffer }, [out.buffer]);                // transfer back
};
```

Transfer only detaches the buffer on the sending side — that's why it's cheap and
why the sender can't touch it afterwards. Instantiate the module once per worker,
not per message.

## Error handling across the boundary

- A WASM **trap** (out-of-bounds, `unreachable`, bad indirect call) surfaces in JS
  as a `WebAssembly.RuntimeError`. Wrap boundary calls in `try/catch`; you can't
  resume a trapped instance — recreate it.
- **Rust**: return `Result<T, JsValue>` so an `Err` becomes a thrown JS exception
  instead of an opaque trap; set `console_error_panic_hook` so panics print a
  stack rather than a bare `unreachable`.
- **Emscripten**: build with `-sASSERTIONS=1` in dev to convert silent corruption
  into named aborts; C++ exceptions need `-fexceptions` (or `-fwasm-exceptions`)
  to propagate rather than trap.
- Never swallow a boundary error — route it through the project's error channels.

## See also

- Where the pointers/exports come from: [emscripten-patterns.md](emscripten-patterns.md), [rust-wasm-patterns.md](rust-wasm-patterns.md)
- Decoding a `RuntimeError`: [debugging.md](debugging.md)
