# Testing and gates — detailed patterns

Detailed code samples and patterns supporting `rules/testing-gates.md` and `rules/testing-authoring.md`.

## Test hang prevention

A test that blocks on a real socket accept/connect, or a real thread sleep, with no bound can hang forever — one such test burned **~40 minutes** of CI as a zombie binary thrashing the build lock.

**The pattern to copy** (from `browser_extension/server/server_io_tests.rs`): round-trip the whole framed IPC + token-auth gate over a `tokio::io::duplex` pair — no socket, no port, no browser.

### Good: in-memory transport, no real socket

```rust
let (client, server) = tokio::io::duplex(4096);
let (s_read, s_write) = tokio::io::split(server);
let task = tokio::spawn(async move { handle_conn(s_read, s_write, state).await });
// … write a frame over `client`, assert the reply. `task.abort()` at the end.
```

### Good: virtual time, never a real sleep

```rust
#[tokio::test(start_paused = true)]
async fn keepalive_ticks_once_per_interval() {
    tokio::time::sleep(Duration::from_secs(20)).await; // advances instantly
    assert_eq!(pings.load(Ordering::SeqCst), 1);
}
```

### Bad: real socket accept with no timeout

```rust
// ⛔ unbounded — hangs forever if no peer connects
#[tokio::test]
async fn accepts_a_connection() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let (sock, _) = listener.accept().await.unwrap();
}
```

### When you must use a real socket

Only when the socket *itself* is the unit under test (ECONNREFUSED, port-in-use, real HTTP framing). Keep it bounded — async gets a timeout, a sync mock server gets an ephemeral `127.0.0.1:0` plus `set_read_timeout` so a leaked accept thread can't wedge the box — and mark the line:

```rust
// test-hang-allow: ephemeral 127.0.0.1:0 one-shot mock; accept thread has
// set_read_timeout(2s), leaked thread dies with the process.
let listener = TcpListener::bind("127.0.0.1:0").unwrap();
```

## Module mocks — the factory pattern

Some test runners register module mocks **globally and persistently**: a mock registered by one file stays active for every file that runs after it. A *partial* mock (`{ addToast }` when the real module also exports `notify`, `celebrate`, …) makes any later-running file that imports a missing export crash with `SyntaxError: Export named 'X' not found` — an order-dependent failure that is green locally and red in CI.

### Correct pattern

Provide the module's **complete export surface** — either a shared `make*Mock()` helper or a spread of the real module:

```ts
const real = await import("@scope/ui")
mock.module("@scope/ui", () => ({ ...real, SOME_ID: stub }))
```

Never a bare partial object literal. Adding a mock for a new module means adding a `make<Name>Mock()` covering every runtime export.

## Process-per-test

Running each test in its own process structurally isolates process-global state: `$HOME` mutation, in-process singletons, projection caches, `OnceLock`-style statics. Adopting a process-per-test runner let a ~2,900-test suite drop `--test-threads=1` as a blanket requirement and run fully in parallel.

**Keep a hang guard.** A per-test slow-timeout that flags at 60s and **TERMINATES and names** the test at 120s ends unbounded multi-minute hangs.

**Serialize only what is genuinely machine-global** — the OS keychain is the classic one, since process-per-test cannot isolate it. Put those tests in one named group with `max-threads = 1`. Everything else runs parallel.

**Check whether your runner runs doctests.** Many do not. If you add a runnable doctest, add an explicit step for it — otherwise it is silently skipped.

## Compile gates and phase ordering

A ratchet that only reads source text has no inherent build dependency. It *inherits* one by being packaged as a compiled test, and then it cannot run until the test targets build — which puts the cheapest, most discriminating check in the repo **behind** the slowest gates.

**Example:** A text scan for a banned credential filename — a string list matched against file contents, no compilation required — failed in **8.2s** and named the defect in one sentence. Because it lived in the compiled architecture binary it ran only after two gates had each burned 300s and been killed, and after a cold workspace test build. The defect was a string in a file written an hour earlier.

**Solution:** Package a source-text check as a script the no-compile phase runs, beside the format and banned-id scans, not as a test.

## Vitest timeout collateral

A test killed by `testTimeout` mid-render keeps running, so a file whose render helper assigns its root handle *after* the `await` never records the container, and `afterEach` cannot detach it. Two live subtrees then carry the same element ids, which defeats jsdom's `#id` fast path — so the *next* test's `container.querySelector("#x")` is null.

**Protect:** assign the root/container handle before any `await`, and sweep body nodes in setup cleanup.
