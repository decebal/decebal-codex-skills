# Auth and production hardening

## JWT with `@fastify/jwt` (asymmetric)

Prefer **asymmetric** signing (ES256 / RS256) in any multi-service setup: the auth issuer holds the private key, every API verifies with the **public** key only — a leaked API host cannot mint tokens. `@fastify/jwt` decorates `app.jwt`, `reply.jwtSign()`, and `req.jwtVerify()`.

```ts
import jwt from "@fastify/jwt";

app.register(jwt, {
  secret: {
    private: process.env.JWT_PRIVATE_KEY!,   // only on the issuer; omit on verify-only services
    public: process.env.JWT_PUBLIC_KEY!,     // PEM public key
  },
  sign: { algorithm: "ES256", expiresIn: "15m" },
  verify: { algorithms: ["ES256"] },          // pin the algo — never trust the token header's alg
});

// verify-only service: pass just the public key
// app.register(jwt, { secret: { public: PUBLIC_KEY }, verify: { algorithms: ["RS256"] } });
```

Pair with the `authenticate` decorator from references/plugins-and-encapsulation.md and gate routes: `{ preHandler: [app.authenticate] }`. **Always pass an explicit `algorithms` allowlist to `verify`** — accepting the token's own `alg` header is the classic JWT confusion attack.

## Rate limiting — `@fastify/rate-limit`

```ts
import rateLimit from "@fastify/rate-limit";
app.register(rateLimit, {
  max: 100,
  timeWindow: "1 minute",
  keyGenerator: (req) => req.user?.tenantId ?? req.ip,   // per-tenant, falling back to IP
  // redis: new Redis(...)   // share the counter across instances in production
});
```

Register it **global** (before routes) for a blanket limit, or inside a plugin for a per-route-group limit — encapsulation applies here too.

## Multi-tenant routing

Two real patterns:

- **Prefix / path:** `app.register(tenantRoutes, { prefix: "/t/:tenant" })` and resolve the tenant in an `onRequest` hook.
- **Host constraint:** Fastify's built-in `constraints` routes by `Host` header, so `tenant-a.example.com` and `tenant-b.example.com` hit different handlers:

```ts
app.get("/dashboard", { constraints: { host: "tenant-a.example.com" } }, handlerA);
```

Resolve and attach the tenant once, early, on a decorated request slot (declare `decorateRequest("tenant", null)`, assign in `onRequest`) so every downstream hook and handler sees it.

## Structured logging with pino + redaction

Fastify's logger **is** pino. Configure `redact` so secrets never reach the logs — a token in a log file is a breach.

```ts
const app = Fastify({
  logger: {
    level: process.env.LOG_LEVEL ?? "info",
    redact: {
      paths: ["req.headers.authorization", "req.headers.cookie", "req.body.password", "*.creditCard"],
      remove: true,     // drop the key entirely rather than printing [Redacted]
    },
  },
});
```

Use `req.log` / `app.log` (child loggers carry the `reqId`) — never `console.log`, which bypasses redaction and level control.

## Error handling — map, do not leak

Set one `setErrorHandler` at the root. This is the two-channel discipline from [error-channels.md](../../../rules/error-channels.md) applied to HTTP: the **user** gets a safe, human, actionable message with a stable machine `code`; the **dev team** gets the full error in the log. A stack trace or raw driver message in the response body is an information-leak bug.

```ts
app.setErrorHandler((error, req, reply) => {
  req.log.error({ err: error, reqId: req.id }, "request failed");   // dev-only channel: full detail

  if (error.validation) {                                            // Ajv/schema failure
    reply.code(400).send({ code: "VALIDATION_ERROR", message: "Check the highlighted fields and try again." });
    return;
  }

  const status = error.statusCode ?? 500;
  reply.code(status).send({
    code: error.code ?? "INTERNAL_ERROR",
    // never echo error.message for 5xx — it can carry a stack/driver string
    message: status >= 500
      ? "Something went wrong on our end. Please try again shortly."
      : error.message,
  });
});
```

Define stable error codes with `@fastify/error`'s `createError(code, message, statusCode)` so callers branch on `code`, not on prose:

```ts
import createError from "@fastify/error";
export const UserNotFound = createError("USER_NOT_FOUND", "User %s does not exist", 404);
// throw new UserNotFound(id)  → statusCode 404, code "USER_NOT_FOUND"
```

Also set `setNotFoundHandler` so 404s return the same `{ code, message }` shape, not Fastify's default text.

## Health, readiness, and graceful shutdown

**Liveness vs readiness are different questions** — liveness is "the process is up", readiness is "it can serve traffic (deps reachable, not shutting down)". Keep two routes:

```ts
app.get("/healthz", async () => ({ status: "ok" }));                    // liveness — cheap, always 200 if up
app.get("/readyz", async (_req, reply) => {                             // readiness — checks deps
  try { await app.db.query("SELECT 1").get(); return { status: "ready" }; }
  catch { reply.code(503); return { status: "degraded" }; }
});
```

`@fastify/under-pressure` can automate readiness (event-loop-delay / memory thresholds, `exposeStatusRoute`).

**Graceful shutdown on SIGTERM** — stop accepting new connections, let in-flight requests drain, run `onClose` hooks (close db, flush), then exit:

```ts
const app = Fastify({
  logger: true,
  return503OnClosing: true,        // new requests during shutdown get 503, not a hang (default)
  forceCloseConnections: "idle",   // close idle keep-alive sockets so close() can resolve
});

for (const signal of ["SIGINT", "SIGTERM"] as const) {
  process.once(signal, async () => {
    app.log.info({ signal }, "shutting down");
    try {
      await app.close();           // resolves after in-flight drain + onClose hooks
      process.exit(0);
    } catch (err) {
      app.log.error({ err }, "error during shutdown");
      process.exit(1);
    }
  });
}
```

Do cleanup in `onClose` hooks (co-located with the resource that opened it), not in the signal handler — `app.close()` runs them in reverse registration order.
