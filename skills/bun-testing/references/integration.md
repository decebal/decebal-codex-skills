# Integration testing with bun:test

HTTP services, database/external-service boundaries, and environment isolation — kept
**un-hangable** per [../../../rules/testing-gates.md](../../../rules/testing-gates.md): no real
listening port left open, no unbounded await, no real sleep. A test that blocks on a
socket can hang a whole CI run as a zombie. Verified against Bun 1.3.14.

## 1. HTTP handlers — test the function, not a port

The cleanest un-hangable HTTP test binds **no port at all**: a `Bun.serve` `fetch`
handler is just `(req: Request) => Response | Promise<Response>`, so factor it out and
call it directly with a `Request`.

```ts
// server.ts
export async function handler(req: Request): Promise<Response> {
  const url = new URL(req.url);
  if (url.pathname === "/health") return Response.json({ ok: true });
  return new Response("not found", { status: 404 });
}
// index.ts — production entrypoint only
// Bun.serve({ port: 3000, fetch: handler });
```

```ts
// server.test.ts
import { test, expect } from "bun:test";
import { handler } from "./server";

test("GET /health returns ok", async () => {
  const res = await handler(new Request("http://x/health"));
  expect(res.status).toBe(200);
  expect(await res.json()).toEqual({ ok: true });
});

test("unknown path is 404", async () => {
  const res = await handler(new Request("http://x/nope"));
  expect(res.status).toBe(404);
});
```

Any base URL works — the handler only reads `url.pathname`. No server, no teardown,
cannot hang.

### When you must exercise the real server

If the behaviour under test lives in Bun's server (routing config, `server.upgrade`,
streaming), bind an **ephemeral port** (`port: 0`), always `stop()` in `afterEach`, and
**bound every fetch** with a timeout so a wedged server fails fast instead of hanging:

```ts
import { afterEach, expect, test } from "bun:test";
let server: ReturnType<typeof Bun.serve> | undefined;
afterEach(() => server?.stop(true)); // true = close active connections too

test("serves over a real socket", async () => {
  server = Bun.serve({ port: 0, fetch: () => new Response("hi") }); // 0 = OS-assigned free port
  const res = await fetch(server.url, { signal: AbortSignal.timeout(2000) }); // bounded — never unbounded
  expect(await res.text()).toBe("hi");
});
```

`AbortSignal.timeout(ms)` is the bound that satisfies the un-hangable rule; `port: 0`
avoids the "port already in use" flake from a hard-coded port.

## 2. Fastify (and Express-like) handlers — `app.inject()`, no port

Fastify's `inject` (via light-my-request) drives the full route/middleware stack
in-memory. Nothing listens, so it is un-hangable by construction — never `app.listen()`
in a test.

```ts
import { test, expect, beforeAll, afterAll } from "bun:test";
import { build } from "./app"; // returns a configured Fastify instance

let app: Awaited<ReturnType<typeof build>>;
beforeAll(async () => { app = build(); await app.ready(); });
afterAll(async () => { await app.close(); });

test("POST /users validates and creates", async () => {
  const res = await app.inject({
    method: "POST",
    url: "/users",
    payload: { name: "Ada" },
  });
  expect(res.statusCode).toBe(201);
  expect(res.json()).toMatchObject({ name: "Ada" });
});
```

`res.statusCode`, `res.json()`, `res.body`, and `res.headers` are available without a
socket. Call `app.close()` in `afterAll` so plugin `onClose` hooks (DB pools, timers)
actually run.

## 3. Database and external-service boundaries

Two honest options — pick per what the test is proving:

**Real logic against an in-memory SQLite** (best for exercising actual queries):

```ts
import { Database } from "bun:sqlite";
import { beforeEach, afterEach, test, expect } from "bun:test";

let db: Database;
beforeEach(() => {
  db = new Database(":memory:");                 // fresh, isolated, no file, no cleanup race
  db.run("CREATE TABLE users (id TEXT PRIMARY KEY, name TEXT)");
});
afterEach(() => db.close());

test("insert then read back", () => {
  db.query("INSERT INTO users VALUES (?, ?)").run("u1", "Ada");
  expect(db.query("SELECT name FROM users WHERE id = ?").get("u1")).toEqual({ name: "Ada" });
});
```

A round-trip like this (write persists, reads back) is a real integration test; a mock
that returns a canned row proves nothing — see
[../../../rules/testing-gates.md](../../../rules/testing-gates.md) on mocks so complete the
assertion can only be true.

**Mock the outbound client** (best when the dependency is a remote HTTP API you must not
call). Use `mock.module` with the FULL export surface — see
[mocking.md](mocking.md); a partial factory crashes a later file with
`Export named 'X' not found`:

```ts
const real = await import("./paymentClient");
mock.module("./paymentClient", () => ({
  ...real,
  charge: mock(() => Promise.resolve({ id: "ch_1", status: "succeeded" })),
}));
```

Do NOT hit a live dev server or a shared DB — a test that passes only because someone
else's server happens to be up is an absence wearing a green badge
([../../../rules/evidence-discipline.md](../../../rules/evidence-discipline.md)).

## 4. Per-test environment isolation

`process.env` is process-global; within a file every test shares it, so a value set by
one test leaks into the next unless you restore it. Snapshot and restore around each
test (Bun auto-loads `.env`, so tests also inherit whatever is on disk — override
explicitly):

```ts
import { beforeEach, afterEach, test, expect } from "bun:test";

let saved: NodeJS.ProcessEnv;
beforeEach(() => { saved = { ...process.env }; });   // shallow snapshot
afterEach(() => { process.env = saved; });           // full restore, incl. deletions

test("reads REGION from env", () => {
  process.env.REGION = "eu-west-1";
  expect(resolveRegion()).toBe("eu-west-1");
});

test("falls back when REGION unset", () => {
  delete process.env.REGION;                          // isolated: the block above is undone
  expect(resolveRegion()).toBe("us-east-1");
});
```

For hard isolation across files, run with `--parallel` (each file gets its own worker
process, so `process.env` mutations can't cross files) or `--isolate`. The save/restore
pattern above is still required for tests *within* one file.
