# Rust → WASM patterns (wasm-pack + wasm-bindgen)

`wasm-bindgen` generates the JS glue and TypeScript types; `wasm-pack` drives the
build, runs `wasm-opt`, and lays out an npm-ready package.

```bash
cargo install wasm-pack           # or: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
rustup target add wasm32-unknown-unknown
```

## Crate setup

`Cargo.toml` — the crate must build a C dynamic library so it links to wasm:

```toml
[lib]
crate-type = ["cdylib", "rlib"]   # cdylib for wasm; rlib so tests/other crates can use it

[dependencies]
wasm-bindgen = "0.2"

[profile.release]
opt-level = "s"    # or "z" for smallest; "s" balances speed/size
lto = true
codegen-units = 1
```

## Build targets — pick by consumer

```bash
wasm-pack build --release --target web --out-dir pkg
```

| `--target` | Consumer | How it loads |
|---|---|---|
| `web` | native ES modules, no bundler | `import init, { fn } from "./pkg/x.js"; await init();` then call `fn()` |
| `bundler` (default) | webpack / Vite / Rollup | `import { fn } from "./pkg";` — bundler handles the `.wasm` |
| `nodejs` | Node `require`/import | synchronous, uses `fs` to read the wasm |
| `no-modules` | plain `<script>`, no import | attaches to a global |
| `deno` | Deno | ES module for Deno |

For `--target web` you must call the default-exported `init()` (it `fetch`es and
instantiates the wasm) before any export; `initSync(bytes)` is the sync variant.

## `#[wasm_bindgen]` — functions, structs, methods

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 { a + b }

// A struct becomes a JS class; methods on the impl become class methods.
#[wasm_bindgen]
pub struct Counter { value: i32 }

#[wasm_bindgen]
impl Counter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Counter { Counter { value: 0 } }

    #[wasm_bindgen(getter)]
    pub fn value(&self) -> i32 { self.value }

    pub fn increment(&mut self) { self.value += 1; }
}

// Run automatically when the module is instantiated.
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once(); // panics → readable console errors
}
```

`&str`/`String` and `Vec<u8>`/`&[u8]` are marshalled automatically (copied across
the boundary). `#[wasm_bindgen(js_name = camelCase)]` renames for JS conventions.

## Passing rich data: `JsValue` and serde

`JsValue` is an opaque handle to any JS value. For plain data structs, convert
through **serde-wasm-bindgen** rather than JSON strings:

```toml
serde = { version = "1", features = ["derive"] }
serde-wasm-bindgen = "0.6"
```

```rust
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Config { name: String, retries: u32 }

#[wasm_bindgen]
pub fn parse_config(input: JsValue) -> Result<JsValue, JsValue> {
    let cfg: Config = serde_wasm_bindgen::from_value(input)?;   // JS object → Rust
    let out = Config { name: cfg.name, retries: cfg.retries + 1 };
    Ok(serde_wasm_bindgen::to_value(&out)?)                     // Rust → JS object
}
```

Returning `Result<T, JsValue>` surfaces the `Err` as a thrown JS exception.

## Calling browser/JS APIs: web-sys and js-sys

- **js-sys** — bindings to ECMAScript built-ins (`Array`, `Object`, `Promise`,
  `Date`, `Math`, typed arrays).
- **web-sys** — bindings to Web APIs (`window`, `Document`, `HtmlCanvasElement`,
  `fetch`, `WebSocket`). Every interface is a **cargo feature** — enable exactly
  what you use, or compile times explode.

```toml
js-sys = "0.3"
web-sys = { version = "0.3", features = ["console", "Window", "Document", "Element"] }
```

```rust
use wasm_bindgen::prelude::*;
use web_sys::console;

#[wasm_bindgen]
pub fn log_hello() {
    console::log_1(&JsValue::from_str("hello from rust"));
}
```

## Closures — passing Rust functions to JS

JS callbacks (event listeners, `setTimeout`) need a `Closure`. The Rust closure
must **outlive** the JS side that holds it, so either keep the `Closure` in a
long-lived owner or `.forget()` it (leaks it deliberately, for lifetime-of-page
handlers):

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::Closure;

let cb = Closure::wrap(Box::new(move |e: web_sys::Event| {
    web_sys::console::log_1(&e);
}) as Box<dyn FnMut(web_sys::Event)>);

element.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
cb.forget(); // hands ownership to the JS runtime; without this it drops and the
             // callback becomes a dangling function that traps when invoked.
```

## Async

`wasm-bindgen-futures` bridges Rust `Future`s and JS `Promise`s:

```rust
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
pub async fn fetch_len(url: String) -> Result<u32, JsValue> {
    let resp = JsFuture::from(web_sys::window().unwrap().fetch_with_str(&url)).await?;
    // ... await resp.text(), etc.
    Ok(0)
}
```

`wasm_bindgen_futures::spawn_local(async { ... })` runs a fire-and-forget future.

## Size and panics

- `console_error_panic_hook` turns the default `unreachable` trap into a readable
  stack in the console — always set it in `start()`.
- Keep dependencies lean; every `web-sys` feature and every crate adds bytes. See
  [optimization.md](optimization.md) and [../../bundle-analysis/SKILL.md](../../bundle-analysis/SKILL.md).

## See also

- Loading the pkg, memory views, workers: [js-wasm-integration.md](js-wasm-integration.md)
- Shrinking `.wasm` with wasm-opt/twiggy: [optimization.md](optimization.md)
