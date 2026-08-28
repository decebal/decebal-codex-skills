---
name: integration-test
description: "Audit integration-test coverage, find untested cross-boundary paths, and write tests matching project conventions. Use when asked to audit tests, check coverage, find gaps, or add integration tests. Tests must be time-bounded and verified to exercise the intended path."
---

# Integration testing

A four-step loop: **SCAN → GAP → WRITE → RUN & VERIFY.** Do not skip to WRITE.
The load-bearing rules live in [../../rules/testing-gates.md](../../rules/testing-gates.md)
and [../../rules/evidence-discipline.md](../../rules/evidence-discipline.md) — read them; this skill applies them.

Distinguish the two kinds up front: a **unit** test exercises one function in isolation; an
**integration** test crosses a boundary (service→store, handler→service, one service→another,
process→file IO). This skill is about the second. Do not "cover a gap" by adding a unit test for a
crossing that has none.

## Step 1 — SCAN (map what exists before adding anything)

Locate the tests and read the conventions off them. Never impose a new style.

- Find the files: `crates/*/tests/*.rs`, sibling `*_tests.rs` / `foo/tests.rs` (Rust);
  `*.spec.ts` / `*.test.ts` (JS/TS). Search runners: `grep -rl 'tokio::test\|#\[test\]' crates/*/tests`.
- Read one existing integration test end to end. Note the harness the project already uses —
  in this repo the CLI suite drives the **built binary** via `assert_cmd` + `predicates` + `tempfile`
  (`crates/fortimus-cli/tests/cli.rs`), and core logic is exercised in `crates/fortimus-core/tests/`.
  Whatever the fixtures, helper functions, and assertion library are, reuse them.
- Identify the runner and how it is invoked (here: `cargo test --workspace`). Note any hang-guard /
  timeout config and any serialized test group.

Write down, per boundary, what is already covered. This map is the input to Step 2.

## Step 2 — GAP (find untested crossings, rank by blast radius)

List every cross-boundary path in the system, then subtract what SCAN found covered. The remainder
is the gap set. **Prioritize by blast radius, not by ease of writing** — a service→store round-trip
that every request depends on beats an easy-to-mock leaf handler.

Look specifically for:
- **service → store**: does a write actually persist and read back? (round-trip, not a mocked return)
- **handler / API contracts**: does the transport edge accept the documented input and emit the
  documented output shape?
- **cross-service flows**: does an event produced by A get consumed by B end-to-end?
- **process → file IO**: CLI arg parsing, file read/write, exit codes.

## Step 3 — WRITE (skeletons that match discovered conventions, and cannot hang)

Generate test skeletons in the project's own style. Place them where the project already puts
integration tests (`crates/<crate>/tests/<area>.rs` here) — **no new inline `#[cfg(test)] mod tests`**.

Every test must be **un-hangable** — this is not optional (see testing-gates.md):

1. **In-memory transport, never a real socket/port/browser.** Drive I/O over a duplex pipe.
2. **Virtual time, never a real sleep.** Assert interval/timeout logic under a paused clock.
3. **Bound every blocking await** in a timeout (or join within a bounded window) so a leak times
   out and names itself instead of hanging forever.

```rust
// GOOD — in-memory transport: no socket, no port
#[tokio::test]
async fn handler_round_trips_a_frame() {
    let (mut client, server) = tokio::io::duplex(4096);
    let (s_read, s_write) = tokio::io::split(server);
    let task = tokio::spawn(async move { handle_conn(s_read, s_write, state).await });
    // write a request frame over `client`, read the reply, assert on it.
    let reply = tokio::time::timeout(Duration::from_secs(5), read_frame(&mut client))
        .await
        .expect("handler replied within 5s"); // bounded — never hangs
    assert_eq!(reply.status, Status::Ok);
    task.abort();
}

// GOOD — virtual time: the 20s sleep advances instantly
#[tokio::test(start_paused = true)]
async fn keepalive_ticks_once_per_interval() {
    let pings = spawn_keepalive();
    tokio::time::sleep(Duration::from_secs(20)).await;
    assert_eq!(pings.load(Ordering::SeqCst), 1);
}

// BAD — real socket accept, unbounded: hangs forever if no peer connects
let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
let (sock, _) = listener.accept().await.unwrap(); // ⛔
```

**Isolate with process-per-test.** Run each test in its own process so process-global state —
`$HOME` mutation, in-process singletons, `OnceLock`-style statics, projection caches — is structurally
isolated instead of serialized. That lets a suite drop a blanket `--test-threads=1` and run fully
parallel. Serialize **only** what is genuinely machine-global (the OS keychain is the classic one) in a
single named group with `max-threads = 1`; everything else runs concurrent. Keep a per-test hang guard
that flags at 60s and terminates-and-names at 120s (testing-gates.md).

**JS/TS analog:** in-memory duplex → a `PassThrough`/paired stream or an in-process client, not a
listening port; virtual time → `vi.useFakeTimers()` / `jest.useFakeTimers()` with
`advanceTimersByTime`, never `await sleep(...)`; bound every await with a timeout. And when mocking a
module, provide its **complete export surface** (spread the real module, `{ ...real, X: stub }`) — a
partial mock fails order-dependently in CI (testing-gates.md).

Allow a real socket **only** when the socket itself is the unit under test (ECONNREFUSED, port-in-use,
HTTP framing); keep it bounded (`set_read_timeout`) and mark the line `// test-hang-allow: <why>`.

## Step 4 — RUN & VERIFY (a green test proves nothing until you confirm it ran the path)

Look at the artifact, not at the badge (evidence-discipline.md).

- **Run it and read the count.** `cargo test --workspace <name> -- --nocapture`. A filter that
  **matches nothing passes instantly** — confirm the run reports `N passed` with N > 0 for the tests
  you added, not `0 filtered out`.
- **Confirm it exercises the boundary.** Break the code under test on purpose (or assert on a value
  only the real path can produce) and watch the test go RED. A test that stays green when the code is
  broken is testing nothing — a mock so complete the assertion is always true, or a path never reached.
- **No dev-server dependence.** A test that passes only because a server is already running on your
  box is red on a clean checkout. It must stand up its own in-memory harness.
- **Watch the exit code, not a pipe's.** `cmd | tail` reports `tail`'s status; capture the runner's own.

## Checklist

- [ ] SCANNED existing tests; conventions (harness, fixtures, assertion lib, runner) written down
- [ ] GAP list built from real cross-boundary paths, ranked by blast radius not ease
- [ ] New tests live where the project puts integration tests (no new inline `#[cfg(test)]` module)
- [ ] Every test is un-hangable: in-memory duplex (no socket), virtual time (no sleep), bounded awaits
- [ ] Process-per-test isolation; only machine-global resources (keychain) serialized in one `max-threads=1` group
- [ ] Any real-socket use is the unit under test, bounded, and marked `// test-hang-allow:`
- [ ] Module mocks provide the complete export surface
- [ ] Ran the suite; the new tests report `passed` with a non-zero count (not `0 filtered out`)
- [ ] Verified each test goes RED when the code under test is broken — it exercises the real path
- [ ] No dependence on an already-running dev server; exit code is the runner's, not a pipe's
