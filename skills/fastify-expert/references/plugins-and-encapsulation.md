# Plugins, encapsulation, and schemas

## The encapsulation model

`register()` builds a **tree of contexts**. A child context inherits everything from its parent (decorators, hooks, schemas registered before it). But anything a child adds stays in the child:

```
root
 ├── decorate("db")            ← visible to everyone below
 ├── register(routesA)         ← context A
 │     └── addHook onRequest   ← runs ONLY for routesA
 └── register(routesB)         ← context B, sibling — does NOT see A's hook
```

Rule: **child sees parent; parent and siblings do not see child.** That is what makes a plugin independently testable — its hooks and decorators cannot bleed into unrelated routes.

## Breaking encapsulation with `fastify-plugin`

When a decorator or hook *must* be visible to the parent scope (a DB pool, an `authenticate` decorator every route calls), wrap the plugin with `fp`. `fp` tells Fastify **not** to create a new context, so registrations land on the caller's scope.

```ts
// plugins/db.ts
import fp from "fastify-plugin";
import { Database } from "bun:sqlite";

export default fp(
  async (app) => {
    const db = new Database("app.sqlite");
    app.decorate("db", db);                    // now app.db is visible app-wide
    app.addHook("onClose", async () => db.close());
  },
  { name: "db", fastify: "5.x" },              // name enables dependency ordering
);
```

```ts
// plugins/auth.ts — depends on nothing but exposes app.authenticate to all routes
import fp from "fastify-plugin";
export default fp(async (app) => {
  app.decorate("authenticate", async (req, reply) => {
    try { await req.jwtVerify(); }
    catch { reply.code(401).send({ code: "UNAUTHENTICATED", message: "Sign in to continue." }); }
  });
}, { name: "auth", dependencies: ["jwt"] });   // load order: jwt plugin first
```

Decision table:

| You are registering | Wrap with `fp`? | Why |
|---|---|---|
| A group of endpoints (`routes/users.ts`) | **No** | keep its hooks/decorators local |
| A shared capability (db, auth, cache) | **Yes** | parent + all routes must see it |
| A plugin others `dependencies`-reference | **Yes** | only named `fp` plugins participate in ordering |

## Autoload layout

`@fastify/autoload` registers every file in a directory as a plugin, in filename order.

```
src/
  app.ts            ← composition root
  plugins/          ← fp-wrapped, load first, shared everywhere
    db.ts
    auth.ts
  routes/           ← plain plugins, encapsulated endpoint groups
    users/
      index.ts      ← becomes prefix /api/users (with options.prefix + folder name)
    health.ts
```

```ts
app.register(autoload, { dir: join(import.meta.dir, "plugins") });
app.register(autoload, {
  dir: join(import.meta.dir, "routes"),
  options: { prefix: "/api" },      // folder structure extends the prefix
  ignorePattern: /.*\.test\.ts$/,    // keep test files out of the route tree
});
```

Load `plugins/` **before** `routes/` so shared decorators exist by the time routes reference them.

## Dependency injection via decorators

Fastify's DI is decorators + encapsulation — no container library. A capability is decorated once (fp-wrapped) and consumed via `app.<name>` or `req.<name>`:

```ts
// consume — the route never imports the db module directly
export default async function users(app) {
  app.get("/:id", async (req) => app.db.query("SELECT * FROM users WHERE id = ?").get(req.params.id));
}
```

For tests, register a stub in place of the real capability — same decorator name, fake implementation — and the routes under test are unchanged:

```ts
app.register(fp(async (a) => a.decorate("db", fakeDb), { name: "db" }));
```

## Schemas: JSON Schema, TypeBox, Zod

**Shared/reusable schemas** — register once with `$id`, reference with `$ref`:

```ts
app.addSchema({ $id: "user", type: "object", properties: { id: { type: "integer" }, name: { type: "string" } } });
app.get("/users/:id", { schema: { response: { 200: { $ref: "user#" } } } }, handler);
```

**TypeBox** — one declaration is both the runtime schema and the static type:

```ts
import { Type, type Static } from "@sinclair/typebox";
import { TypeBoxTypeProvider } from "@fastify/type-provider-typebox";

const app = Fastify().withTypeProvider<TypeBoxTypeProvider>();

const CreateUser = Type.Object({ name: Type.String({ minLength: 1 }), email: Type.String({ format: "email" }) });
type CreateUser = Static<typeof CreateUser>;

app.post("/users", { schema: { body: CreateUser } }, async (req) => {
  req.body.name; // typed as string, and validated at runtime — no cast
});
```

**Zod** — via a type provider (Zod is not JSON Schema, so it needs a compiler bridge):

```ts
import { serializerCompiler, validatorCompiler, type ZodTypeProvider } from "fastify-type-provider-zod";
app.setValidatorCompiler(validatorCompiler);
app.setSerializerCompiler(serializerCompiler);
const typed = app.withTypeProvider<ZodTypeProvider>();
typed.post("/users", { schema: { body: z.object({ name: z.string() }) } }, handler);
```

Remember the serialization-stripping gotcha from SKILL.md §2: whatever type provider you use, **the response schema decides which fields leave the process.** Add a field to the handler's return without adding it to the response schema and it silently never reaches the client.
