# Testing and gates

## Know what actually enforces anything

Before trusting a green badge, find out whether a check can BLOCK a merge. On a
private repo under a free org plan, protected branches and rulesets are unavailable
**by plan, not by misconfiguration** — every CI check is advisory and a human can
merge straight past a red run.

> A green CI badge can mean **"someone would have seen it"** rather than **"it could
> not have merged."**

Where that is true, **the pre-push hook is not belt-and-braces — it is the
braces**, and that is why the hooks must be strict, wrapped in timeouts, and never
bypassed. `--no-verify` and skip env vars are forbidden precisely because there is
no second line behind them.

Two corollaries that have already cost real defects:

- **"Wired into CI" is not "enforced".** Wiring a gate into a workflow buys
  visibility. Still worth doing — a gate nobody can see is worse — but do not treat
  a workflow addition as closing a hole.
- **A gate that runs in neither CI nor pre-push runs nowhere at all**, and is
  indistinguishable from one that passes. This has happened more than once: an
  admin test suite that no gate invoked, and a whole backend suite skipped by the
  release workflow on the strength of a green quality workflow that ran zero
  backend tests.

## Tests that cannot fail

The worst test is not a failing one. Watch for:

- a suite whose filter matches nothing (it "passes" instantly),
- a test hitting a dev server someone else happens to be running,
- a piped command whose exit code belongs to the pipe,
- a mock so complete the assertion can only be true.

Each is an absence wearing a green badge — see
[evidence-discipline.md](evidence-discipline.md).

## Process-per-test beats a global serial flag

Running each test in its own process structurally isolates the process-global state
that suites usually serialize for: `$HOME` mutation, in-process singletons,
projection caches, `OnceLock`-style statics. Adopting a process-per-test runner let
a ~2,900-test suite drop `--test-threads=1` as a blanket requirement and run fully
in parallel.

- **Keep a hang guard.** A per-test slow-timeout that flags at 60s and
  **TERMINATES and names** the test at 120s ends unbounded multi-minute hangs.
- **Serialize only what is genuinely machine-global** — the OS keychain is the
  classic one, since process-per-test cannot isolate it. Put those tests in one
  named group with `max-threads = 1`. Everything else runs parallel.
- **Existing `#[serial]`-style annotations are harmless** under process-per-test and
  keep the fallback runner working. Don't bulk-remove them.
- **Check whether your runner runs doctests.** Many do not. If you add a runnable
  doctest, add an explicit step for it — otherwise it is silently skipped.

## Shard in CI, run whole locally

Compile the test binaries **once** into an archive, then fan the archive out across
runners with a deterministic partition (`hash:i/N`). The aggregate check is green
only if every shard is.

**A test group must never split across machines** — a group serializes only within
one process-set. Run the whole group on ONE shard and EXCLUDE it from the
partitions. If your config expresses that in more than one place (the group
override plus each profile's default filter), say so in the file: adding one
group-member test then means updating all of them.

Coverage runs as a single, deliberately **unsharded** instrumented pass.

## Tests must be un-hangable

A test that blocks on a real socket accept/connect, or a real thread sleep, with no
bound can hang forever — one such test burned **~40 minutes** of CI as a zombie
binary thrashing the build lock. Two backstops:

- **Runtime** — the runner's per-test slow-timeout terminates and names it.
- **Authoring** — `gates/sh/check-test-hangs.sh` FAILS when a test file introduces
  a real socket `bind`/`connect` or a thread sleep without an inline allow-marker.

The convention:

1. **Drive I/O code over an in-memory duplex pipe** — no socket, no port, no
   browser. A framed IPC protocol plus its auth handshake round-trips fine over a
   `duplex` pair.
2. **Test time-based logic under virtual time** (`start_paused`, fake timers) —
   asserting "one ping per 20s interval" then takes microseconds. Never a real
   wall-clock sleep.
3. **Bound every blocking await** in a timeout, or join the task inside a bounded
   window, so a leak times out instead of hanging.

```rust
// GOOD — in-memory transport, no real socket
let (client, server) = tokio::io::duplex(4096);
let (s_read, s_write) = tokio::io::split(server);
let task = tokio::spawn(async move { handle_conn(s_read, s_write, state).await });
// … write a frame over `client`, assert the reply. `task.abort()` at the end.

// GOOD — virtual time, never a real sleep
#[tokio::test(start_paused = true)]
async fn keepalive_ticks_once_per_interval() {
    tokio::time::sleep(Duration::from_secs(20)).await; // advances instantly
    assert_eq!(pings.load(Ordering::SeqCst), 1);
}

// BAD — real socket accept with no timeout: hangs forever if no peer connects
#[tokio::test]
async fn accepts_a_connection() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let (sock, _) = listener.accept().await.unwrap(); // ⛔ unbounded
}
```

**Allowlist** only when the socket *itself* is the unit under test (ECONNREFUSED,
port-in-use, real HTTP framing). Keep it bounded — async gets a timeout, a sync mock
server gets an ephemeral `127.0.0.1:0` plus `set_read_timeout` so a leaked accept
thread can't wedge the box — and mark the line:

```rust
// test-hang-allow: ephemeral 127.0.0.1:0 one-shot mock; accept thread has
// set_read_timeout(2s), leaked thread dies with the process.
let listener = TcpListener::bind("127.0.0.1:0").unwrap();
```

## Module mocks need a complete-surface factory

Some test runners register module mocks **globally and persistently**: a mock
registered by one file stays active for every file that runs after it. A *partial*
mock (`{ addToast }` when the real module also exports `notify`, `celebrate`, …)
makes any later-running file that imports a missing export crash with
`SyntaxError: Export named 'X' not found` — an order-dependent failure that is green
locally and red in CI.

Rules, worth gating:

1. Module mocking may appear **only** in test files.
2. The factory must provide the module's **complete export surface** — either a
   shared `make*Mock()` helper kept next to the tests, or a spread of the real
   module:
   ```ts
   const real = await import("@scope/ui")
   mock.module("@scope/ui", () => ({ ...real, SOME_ID: stub }))
   ```
3. Never pass a bare partial object literal.

Adding a mock for a new module means adding a `make<Name>Mock()` covering every
runtime export.

## Test file conventions

- **No new inline `#[cfg(test)] mod tests` blocks.** Put unit tests in a sibling
  `foo_tests.rs` or `foo/tests.rs`, integration tests in `tests/`. Grandfather the
  existing ones in a closed allowlist and migrate on touch.
- **Scope the pre-push run to the changed modules**; let CI run the whole suite.
- **Detect comment-only diffs and skip the compile gates** —
  `gates/rust/rust-effective-diff` exits 1 when a change is
  comment/doc/whitespace-only.
