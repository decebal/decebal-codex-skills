---
name: fastify-expert
description: "Build production-grade Node or Bun APIs with Fastify. Use for plugin encapsulation, routes, JSON Schema, TypeBox or Zod validation, lifecycle hooks, decorators, JWT auth, rate limiting, graceful shutdown, error handling, response serialization, and app.inject integration tests."
---

# Fastify Expert

Fastify for Node/Bun API backends. This file is the actionable core; deep detail lives in the two references. Use `bun`/`bunx` for commands (Fastify runs unmodified on Bun).

```bash
bun add fastify
bun add @fastify/autoload @fastify/jwt @fastify/rate-limit   # as needed
bun add -d @sinclair/typebox @fastify/type-provider-typebox  # if using TypeBox
```

## 1. Route organization — a plugin is the unit of encapsulation

Every `register()` creates a **new encapsulated context**: hooks, decorators, and plugins added inside are visible only to that subtree, not to the parent or siblings. This is the core mental model — one plugin = one isolated scope you can reason about and test alone.

Two ways to compose them:

- **Plugin-based (explicit):** a route file exports `async (fastify, opts) => { fastify.get(...) }` and the composition root `register`s it under a prefix. Full control over order and options.
- **File-based (`@fastify/autoload`):** point it at a directory and it registers every file as a plugin. Convention over wiring.

```ts
// app.ts — composition root
import Fastify from "fastify";
import autoload from "@fastify/autoload";
import { join } from "node:path";

export function build() {
  const app = Fastify({ logger: true });
  app.register(autoload, { dir: join(import.meta.dir, "plugins") });          // shared: db, auth
  app.register(autoload, { dir: join(import.meta.dir, "routes"), options: { prefix: "/api" } });
  return app;
}
```

Rule of thumb: **`plugins/` holds cross-cutting capabilities** (must be `fastify-plugin`-wrapped to escape encapsulation), **`routes/` holds encapsulated endpoint groups**. See references/plugins-and-encapsulation.md.

## 2. Schema validation + serialization

Attach a `schema` to a route. Fastify validates the **input** with Ajv (JSON Schema) and serializes the **output** with `fast-json-stringify`.

```ts
app.post("/users", {
  schema: {
    body: { type: "object", required: ["name"], properties: { name: { type: "string" } } },
    response: {
      201: { type: "object", properties: { id: { type: "integer" }, name: { type: "string" } } },
    },
  },
}, async (req, reply) => {
  const user = await createUser(req.body);   // req.body is validated
  reply.code(201);
  return user;
});
```

- **Response schemas make Fastify fast** — `fast-json-stringify` compiles a serializer instead of calling `JSON.stringify`, often 2–3× faster.
- **GOTCHA — the schema also STRIPS fields.** Any property on the returned object that is **not declared in the response schema is silently dropped** from the payload — no error, no log. If a field "disappears" from a response, the response schema is the first suspect. It is also a feature: declare exactly the public shape and secrets/internal fields never leak.
- **TypeBox** gives one source of truth for runtime schema + static type: `Fastify().withTypeProvider<TypeBoxTypeProvider>()`, then `Type.Object({...})` as the schema. **Zod** works via a type-provider (`fastify-type-provider-zod`: set its `validatorCompiler`/`serializerCompiler`). Details + examples in references/plugins-and-encapsulation.md.

## 3. Request lifecycle hooks — in order

Each request flows through these; register with `app.addHook(name, fn)`. Pick the earliest hook that has the data you need.

| Hook | Runs | Use it for |
|---|---|---|
| `onRequest` | first, before body parsed | auth, rate-limit checks, request-scoped setup |
| `preParsing` | before body parse | transform/decompress the raw stream |
| `preValidation` | after parse, before schema validation | mutate payload prior to validation |
| `preHandler` | after validation, before handler | authorization, load tenant/user |
| `preSerialization` | after handler, before serialize | reshape the returned payload |
| `onSend` | after serialize, before bytes sent | tweak the serialized body/headers |
| `onResponse` | after response fully sent | metrics, audit logging |
| `onError` | when an error is sent | error-specific side effects (not the mapper) |
| `onTimeout` | connection times out (`connectionTimeout`) | cleanup on timeout |

`onRequest`/`preHandler` are where 90% of auth and tenant resolution goes. Body is **not** available until after `preValidation`.

## 4. Decorators — extend the instance, request, reply

```ts
app.decorate("db", database);              // app.db everywhere in this scope
app.decorateReply("sendError", function (code, msg) { /* ... */ });

// PERF: initialize a decorated request prop to null, populate in a hook.
app.decorateRequest("user", null);         // fixes the object shape at construction
app.addHook("preHandler", async (req) => { req.user = await loadUser(req); });
```

**Why `null` first:** declaring the property up front means every `Request` is created with the same V8 hidden class (one shape), so property access stays monomorphic and fast. Assigning a fresh object per request via `decorateRequest("user", {})` instead **shares one object across all requests** (a correctness bug) and deoptimizes the shape. Always: declare the slot with a primitive/`null`, assign the real value in `onRequest`/`preHandler`.

## 5. Encapsulation and how to break it

A plain plugin's decorators/hooks do **not** leak to the parent — great for isolation, wrong when you want a shared `db` or `authenticate` visible app-wide. Wrap such a plugin in **`fastify-plugin`** (`fp`) to skip the new context so its decorators register on the parent scope:

```ts
import fp from "fastify-plugin";
export default fp(async (app) => { app.decorate("db", await connect()); }, { name: "db" });
```

Decision: **endpoint group → plain plugin (keep it encapsulated). Shared capability → `fp`-wrapped (break encapsulation on purpose).** Full model, autoload layout, and DI-via-decorators in references/plugins-and-encapsulation.md.

## 6. Testing — `app.inject()`, no socket, un-hangable

`app.inject()` (light-my-request) dispatches a request through the full lifecycle **without binding a port** — no real socket, so tests cannot hang on accept/connect and run in parallel. Per [testing-gates.md](../../rules/testing-gates.md), this is exactly the in-memory-transport pattern to prefer over a listening server.

```ts
import { test, expect } from "bun:test";
import { build } from "../src/app";

test("POST /api/users creates a user", async () => {
  const app = build();
  await app.ready();                         // ensure plugins loaded
  const res = await app.inject({ method: "POST", url: "/api/users", payload: { name: "Ada" } });
  expect(res.statusCode).toBe(201);
  expect(res.json()).toMatchObject({ name: "Ada" });
  await app.close();                          // teardown: runs onClose hooks
});
```

Build the app in a factory (`build()`), `ready()` in setup, `close()` in teardown, and mock shared capabilities by registering a stub `fp` plugin instead of the real one.

## 7. Auth, errors, and production

JWT (`@fastify/jwt`, asymmetric ES256/RS256), rate limiting, graceful shutdown, health/readiness, pino redaction, multi-tenant routing, and `setErrorHandler` are in references/auth-and-production.md.

**Error handling is not optional here:** a thrown error must never send a stack trace to the client. Map it to a human, safe message + a stable `code`; log the raw detail server-side. This is the two-channel rule — user-actionable vs dev-only — from [error-channels.md](../../rules/error-channels.md), applied at `setErrorHandler`.
