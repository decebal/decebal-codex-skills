# bun:test mocking in depth

Everything here is verified against Bun 1.3.14. Import from `bun:test`:

```ts
import { mock, spyOn } from "bun:test";
```

## `mock()` — a mock function

`mock(impl?)` returns a callable that records every invocation and lets you script
return values. The recorded history lives on `.mock`.

```ts
const fetchUser = mock((id: string) => ({ id, name: "stub" }));

fetchUser("a");
fetchUser.mock.calls;        // [["a"]]           — args of each call
fetchUser.mock.results;      // [{ type: "return", value: {...} }]
fetchUser.mock.calls.length; // 1

expect(fetchUser).toHaveBeenCalledTimes(1);
expect(fetchUser).toHaveBeenCalledWith("a");
```

### Scripting behaviour — the `mock*` chain

```ts
const fn = mock();
fn.mockReturnValue(42);            // every call returns 42
fn.mockReturnValueOnce(1);         // next call only, then falls back
fn.mockImplementation((x) => x*2); // replace the whole body
fn.mockImplementationOnce(() => 9);// next call only
fn.mockResolvedValue(user);        // async: resolves to user
fn.mockRejectedValue(new Error()); // async: rejects
```

`*Once` variants queue in order and are consumed one per call; when the queue drains,
the base implementation (or `mockReturnValue`) applies.

### clear vs reset vs restore — they reset DIFFERENT things

This is the most common bun:test mistake. The three are a ladder, each doing strictly
more than the one above (all verified empirically):

| Method | Clears call history (`.mock.calls`/`.results`) | Removes the implementation | Restores the ORIGINAL (spyOn only) |
|---|---|---|---|
| `mockClear()` | yes | no — impl still runs | no |
| `mockReset()`  | yes | yes — calls now return `undefined` | no |
| `mockRestore()`| yes | yes | yes — re-installs the real method |

```ts
const fn = mock((x: number) => x + 1);
fn(1);
fn.mockClear();  // .mock.calls === [] but fn(1) still === 2
fn.mockReset();  // fn(1) now === undefined
```

`mockRestore()` only has a real effect on a mock created by `spyOn` (it puts the
original method back). On a standalone `mock()` it behaves like `mockReset()`.

**Where to put them:** call `mockClear()`/`mockReset()` in `beforeEach`/`afterEach` so
call history from one test never leaks into the next — a stale `.mock.calls` is a
silent false pass. Globals reset everything at once:

- `mock.restore()` — restores all `spyOn` spies to their originals (call in
  `afterEach`/`afterAll`; the analogue of Jest `restoreAllMocks`).
- `mock.clearAllMocks()` — clears call history on all mocks.

## `spyOn` — wrap a real object method

`spyOn(obj, "method")` records calls while **still calling the original** until you
override it. Ideal for asserting a side-effecting method was called without stubbing it.

```ts
const logger = { warn: (m: string) => process.stderr.write(m) };
const spy = spyOn(logger, "warn");           // original still runs
doThing(logger);
expect(spy).toHaveBeenCalledWith("low disk");

spy.mockReturnValue(undefined);              // now suppress the real write
// ...
spy.mockRestore();                           // put logger.warn back (or use afterEach)
```

Restore spies (`spy.mockRestore()` or `mock.restore()`), because the patch is a mutation
of a shared object that outlives the test.

## `mock.module()` — override a whole module

`mock.module(specifier, factory)` replaces a module's exports for subsequent imports.
Two ways it is **fundamentally different from `jest.mock`** — both cost real time when
misunderstood:

1. **Registration is GLOBAL and PERSISTS for the entire `bun test` run.** A mock
   registered in one file stays active for every file that runs *after* it in the same
   process. It is not scoped to the file, and it is not undone at end-of-file. Use
   `--isolate` (fresh global per file) or restore in `afterEach` when a mock must not
   leak. This is the exact order-dependent trap in
   [../../../rules/testing-gates.md](../../../rules/testing-gates.md): green locally, red in CI
   (or vice-versa) purely from file execution order.

2. **No hoisting magic.** `jest.mock(...)` is hoisted above the imports; `mock.module`
   is not. It must execute **before** the code under test imports the target, so either
   register it at the very top of the test file before importing the subject, or in a
   preload (`bunfig.toml` `[test] preload = ["./setup.ts"]`).

### CRITICAL: provide the COMPLETE export surface

A **partial** factory silently drops every export you didn't list. A later-running file
that imports one of the dropped names crashes with
`SyntaxError: Export named 'X' not found` — and because `mock.module` is global +
persistent, that crash lands in a *different* file than the one with the bad mock. This
is [../../../rules/testing-gates.md](../../../rules/testing-gates.md)'s complete-surface rule.

Spread the real module and override only what you need — verified to preserve default +
named exports:

```ts
// GOOD — full surface preserved, one export stubbed
const real = await import("./notifier");
mock.module("./notifier", () => ({
  ...real,
  send: mock(() => Promise.resolve({ ok: true })),
  // notify, celebrate, EVENTS, default … all still present via ...real
}));
```

```ts
// BAD — a bare partial literal. Any later file importing `notify` from
// "./notifier" now throws "Export named 'notify' not found".
mock.module("./notifier", () => ({ send: mock() }));
```

For a module reused across many test files, factor the factory into a
`makeNotifierMock()` helper kept next to the tests, and have every file call it — one
place to keep the surface complete.

### default + named exports

A module's `export default foo` is the `default` key in the factory object; named
exports are their own keys. The spread carries both, so you only name overrides:

```ts
const real = await import("./config");
mock.module("./config", () => ({
  ...real,                       // keeps `default` and all named exports
  FLAGS: { beta: true },         // override one named export
  default: () => real.default({ env: "test" }), // wrap the default export
}));
```

## Jest-compat shim

Bun also exposes a `jest` object (`import { jest } from "bun:test"`) with `jest.fn()`,
`jest.spyOn`, `jest.restoreAllMocks()`, etc., to ease migration. Prefer the native
`mock` / `spyOn` for new tests; the compat surface is for porting existing suites with
minimal churn.
