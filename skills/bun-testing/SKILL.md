---
name: bun-testing
description: "Write and run tests with Bun's built-in bun:test runner. Use for test organization, lifecycle hooks, assertions, snapshots, coverage, mocking, Turborepo caching, flaky tests, or Jest and Vitest migrations. See references/mocking.md for mocks and references/integration.md for HTTP, database, and environment-isolation tests."
---

# Bun testing (bun:test)

The Bun test runner is Jest-compatible in surface but NOT in mechanics — the two
places that bite are module mocking (global + persistent, no hoisting) and coverage
threshold keys (plural, silently ignored if singular). This skill is the fast path;
depth lives in the two reference files. The load-bearing repo rules are
[../../rules/testing-gates.md](../../rules/testing-gates.md) (un-hangable tests, mocks
that can't fail, what actually enforces a gate) and
[../../rules/evidence-discipline.md](../../rules/evidence-discipline.md) (a green run
that ran zero tests is not a pass — read the count, not the badge).

Verified against Bun 1.3.14. Run everything with `bun` / `bunx`, never npm/npx.

## 1. Organize

Import from `bun:test`. Colocate as `*.test.ts` / `*.spec.ts` next to source, or under
`__tests__/`. A file must match `*.test.*`, `*_test.*`, `*.spec.*`, or `*_spec.*`
(e.g. `cart.test.ts` or `cart_test.ts`) or the runner skips it silently.

```ts
import { describe, it, test, expect } from "bun:test";

describe("cart", () => {
  it("sums line items", () => {
    expect(total([{ price: 2 }, { price: 3 }])).toBe(5);
  });
});
```

- `it` and `test` are aliases. Nest `describe` freely.
- Variants: `test.skip`, `test.only`, `test.todo` (shown only with `--todo`),
  `test.failing` (passes when the body throws), `test.if(cond)` / `test.skipIf(cond)`,
  and table tests `test.each([...])` / `describe.each([...])`.

### Lifecycle

```ts
import { beforeAll, beforeEach, afterEach, afterAll } from "bun:test";

beforeAll(() => {/* once, before the first test in scope */});
beforeEach(() => {/* before every test in scope */});
afterEach(() => {/* after every test — put cleanup here so a failing test still cleans up */});
afterAll(() => {/* once, after the last test in scope */});
```

Declaring these inside a `describe` scopes them to that block; at file top level they
wrap the whole file. Return a promise (or use `async`) and Bun awaits it.

### Selecting and parallelism

- **By file** — positional patterns: `bun test cart` runs files whose path contains
  `cart`; `bun test ./src/cart.test.ts` runs one exact file. (There is no `--filter`
  on `bun test`; `--filter` is a `bun run` workspace flag.)
- **By test name** — `-t` / `--test-name-pattern` takes a **regex** matched against the
  full `describe > it` name: `bun test -t "sums|adds"`.
- **Serial by default.** Tests within a file run in definition order, one at a time.
  Opt into concurrency with `test.concurrent(...)` (or `--concurrent` for all),
  bounded by `--max-concurrency=N`. Test *files* run in one process by default; run
  them in parallel worker processes with `--parallel[=N]` (implies `--isolate`).
- **`--isolate`** gives each file a fresh global object so a leaked handle or a
  registered `mock.module` from one file can't bleed into the next — reach for it the
  moment you see an order-dependent failure (green locally, red in CI).
- Other useful flags: `--watch` (re-run on change), `--bail[=N]`, `--rerun-each=N` and
  `--randomize`/`--seed=N` (flake hunting), `--changed` (only files git says changed),
  `--only-failures`, `--timeout=<ms>` (per-test, default 5000).

## 2. Assert (`expect`)

```ts
expect(x).toBe(1);                    // Object.is
expect({ a: 1 }).toEqual({ a: 1 });   // deep, ignores undefined props
expect({ a: 1, b: 2 }).toMatchObject({ a: 1 }); // subset
expect([1, 2, 3]).toContain(2);
expect(v).toBeDefined();  expect(v).toBeNull();  expect(v).toBeTruthy();
expect(fn).toHaveBeenCalledWith("arg");
```

### Throwing — the sync vs async split matters

```ts
// SYNC: pass a FUNCTION, never the already-thrown call
expect(() => parse("bad")).toThrow(SyntaxError);
expect(() => parse("bad")).toThrow(/unexpected/);

// ASYNC: assert on the rejection, and AWAIT it (a missing await = a test that can't fail)
await expect(loadUser("nope")).rejects.toThrow("not found");
await expect(loadUser("ok")).resolves.toEqual({ id: "ok" });
```

`await expect(promise).resolves` / `.rejects` unwraps the promise, then chains a normal
matcher. Forgetting the `await` makes the assertion a dangling promise the runner never
checks — see [../../rules/testing-gates.md](../../rules/testing-gates.md) on tests that
pass by never running.

### Snapshots

```ts
expect(node).toMatchInlineSnapshot();   // Bun writes the value into this call on first run
expect(node).toMatchSnapshot();         // stored under __snapshots__/<file>.snap
```

Inline snapshots keep the expected value in the test file (best for small, reviewable
shapes); file snapshots suit large payloads. Regenerate with `bun test -u`
(`--update-snapshots`) — and actually read the resulting diff, because `-u` will
happily bless a regression.

## 3. Coverage & CI

```bash
bun test --coverage                              # text report to stdout
bun test --coverage --coverage-reporter=lcov     # lcov for CI upload (writes coverage/)
```

Persist the policy in `bunfig.toml` so every run and the pre-push gate agree:

```toml
[test]
coverage = true
# Scalar = a single threshold for all metrics:
coverageThreshold = 0.9
# OR per-metric — keys are PLURAL. Singular (line/function/statement) is SILENTLY IGNORED:
coverageThreshold = { lines = 0.9, functions = 0.9 }  # `statements` is accepted but NOT enforced by Bun today
coverageReporter = ["text", "lcov"]   # default is ["text"]
coverageDir = "coverage"
coverageSkipTestFiles = true          # exclude *.test.ts from the denominator
```

When any threshold is unmet, `bun test` **exits non-zero** — verified: it fails the run,
which is what makes it a real gate rather than a report. Because a green CI badge on a
free-plan repo may not block a merge (see
[../../rules/testing-gates.md](../../rules/testing-gates.md)), wire `bun test --coverage`
into the pre-push hook, not CI alone.

### Turborepo caching

Cache the test task so unchanged packages don't re-run. Coverage output is an artifact,
so declare it as an output or Turbo will cache an empty result:

```json
{
  "tasks": {
    "test": {
      "dependsOn": ["^build"],
      "outputs": ["coverage/**"],
      "cache": true
    }
  }
}
```

Run with `bunx turbo run test`. Turbo keys the cache on file hashes + task inputs; a hit
replays stdout and the `coverage/` dir without spawning Bun. Add env vars the tests read
to `globalEnv`/`env` so a cache hit can't mask an env-dependent result — the same
"absence that looks like a pass" trap as above.

## Depth

- **Mocking** — `mock()`, `spyOn`, `mock.module`, and the clear/reset/restore
  distinctions (they are not interchangeable): [references/mocking.md](references/mocking.md).
- **Integration** — HTTP against `Bun.serve`, Fastify `app.inject()`, DB/service mocking,
  per-test `process.env` isolation, all kept un-hangable:
  [references/integration.md](references/integration.md).
