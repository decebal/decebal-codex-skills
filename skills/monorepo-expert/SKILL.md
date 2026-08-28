---
name: monorepo-expert
description: "Develop Turborepo and Bun workspaces. Use for turbo.json pipelines, local or remote caching, package filters, shared tooling, workspace dependencies, cross-service environment and ports, affected-package CI, turbo prune, cache misses, or monorepo Docker builds."
---

# Monorepo Expert (Turborepo + Bun)

Interactive companion to the static `templates/monorepo.md`. That template gives a
project its structure; this skill tells you what to DO when the pipeline, cache, or
workspace graph misbehaves. Bun is the only package manager here — `bun` / `bunx`,
never `npm`/`pnpm`/`npx`.

## 0. Orient first

```bash
bunx turbo --version                              # 2.x uses "tasks"; 1.x used "pipeline"
grep -E '"(tasks|pipeline)"' turbo.json           # which key is this repo on?
cat package.json | grep -A3 '"workspaces"'        # workspace globs (apps/*, packages/*)
```

`turbo.json` is the pipeline definition. **In Turborepo 2.0 the top-level task map
was renamed `"pipeline"` → `"tasks"`.** On 2.x a top-level `"pipeline"` key is
REJECTED — the run fails with `found "pipeline" field instead of "tasks"` until you
migrate (`bunx @turbo/codemod migrate`, or rename it by hand). 1.x used `pipeline`;
use `tasks` for anything on 2.x. Everything below uses `tasks`.

## 1. The task pipeline

Each entry under `tasks` describes ONE task name (`build`, `dev`, `test`, …) and
how it relates to the same task in other packages.

| Key | What it does |
|---|---|
| `dependsOn` | Tasks that must finish first. `"^build"` = run `build` in all *upstream dependencies* before this package's task. A bare `"build"` = a task in the SAME package. |
| `outputs` | Glob(s) of files to cache on success (`dist/**`, `.next/**` but not `.next/cache/**`). **This is the cache contract — see §3.** |
| `inputs` | Globs that define what counts as a change for the hash. Default is the package's git-tracked files; narrow it to skip unrelated churn. |
| `cache` | `true` by default. Set `false` for `dev` and anything with side effects. |
| `persistent` | `true` for long-running tasks (dev servers, watchers). A task that `dependsOn` a persistent task is a config error — Turbo refuses to let a one-shot task wait on something that never exits. |
| `env` / `passThroughEnv` | Env vars that affect the hash / are passed through without affecting it. A missing `env` entry is the classic "stale cache in CI" bug. |
| `outputLogs` | `full` \| `hash-only` \| `new-only` \| `errors-only` \| `none`. |

Full annotated example that actually parses: **[references/turbo-json.md](references/turbo-json.md)**.

## 2. Filtering — run only what you need

```bash
bunx turbo run build --filter=@repo/api           # just that package
bunx turbo run test  --filter=...[HEAD^1]          # packages changed since HEAD^1 + their dependents
bunx turbo run lint  --filter=...[origin/main]     # affected vs the trunk — the CI pattern
bunx turbo run build --filter=@repo/ui...          # @repo/ui AND its dependencies (upstream)
bunx turbo run build --filter=...@repo/ui          # @repo/ui AND its dependents (downstream)
bunx turbo run dev   --filter=@repo/web            # scope a dev server to one app
```

**Get the `...` direction right — it is the opposite of what most people guess:**

| Syntax | Selects |
|---|---|
| `pkg` | only `pkg` |
| `pkg...` (trailing) | `pkg` **and its dependencies** (what pkg needs) |
| `...pkg` (leading) | `pkg` **and its dependents** (what needs pkg) |
| `...[<ref>]` | packages changed since `<ref>` **and their dependents** — why CI uses the leading form: rebuild what changed plus everything downstream it could break |

## 3. Caching — the outputs glob is the whole game

Turbo hashes a task's inputs (files + `env` + `dependsOn` outputs) and, on a hit,
**replays the cached `outputs` and the logs instead of running the task.**

- **Cacheable** = `cache` is not `false` AND the task is deterministic from its
  declared inputs. `dev` is never cacheable; `build`/`test`/`lint`/`typecheck` are.
- **The `outputs` glob is a correctness contract, not an optimization.** Too NARROW
  and a real artifact is silently missing after a cache hit (a false hit — the build
  "succeeded" but `dist/` is incomplete). Too BROAD and you cache `node_modules` or a
  `.next/cache` that poisons the next run. Match `outputs` to exactly what the task
  writes — this is the single most common turbo.json defect.
- **Local cache** lives in `.turbo/cache/`. **Remote cache** shares hits across
  machines and CI:
  ```bash
  bunx turbo login && bunx turbo link            # Vercel-hosted remote cache
  # self-hosted / CI: export TURBO_API, TURBO_TOKEN, TURBO_TEAM
  ```
  In CI, remote cache turns "rebuild everything" into "download last green
  artifact". Verify a hit rather than trusting it — see §6 and
  [references/diagnostics.md](references/diagnostics.md).

## 4. Workspace patterns

- **Workspaces** are declared once in the root `package.json`
  (`"workspaces": ["apps/*", "packages/*"]`); Bun resolves internal packages via the
  `workspace:*` protocol (`"@repo/ui": "workspace:*"`).
- **Internal (unpublished) packages carry `"private": true`.** This is what keeps a
  shared package off npm and lets it stay `workspace:*`. Published packages omit it
  and get a real semver.
- **Shared tooling is a package, not copy-paste.** A `@repo/tsconfig` (base
  `tsconfig.json` others `extends`), a `@repo/biome-config`, a `@repo/env` — each is a
  private package the apps depend on, so one edit propagates and Turbo tracks it in
  the graph. Import through the package's barrel (`index.ts`), never a deep path into
  its internals — see [../../rules/layer-boundaries.md](../../rules/layer-boundaries.md).
- **Versioning:** *fixed/locked* (every package moves to one version together — one
  release train) vs *independent* (each package versions on its own). Internal-only
  repos usually pin everything to `workspace:*` and never publish; the choice only
  bites once you publish.
- Keep the graph lean: an internal package that nothing imports is dead weight, and a
  second major of a shared dep compiles twice — see
  [../../rules/dependency-hygiene.md](../../rules/dependency-hygiene.md).

## 5. Cross-cutting concerns

- **One env schema for the whole repo.** Put a Zod schema in `@repo/env` that
  `parse`s `process.env` at import and exports the typed, validated object. Every app
  imports it, so a missing/renamed var fails fast at boot instead of `undefined` deep
  in a handler. Register those vars in `turbo.json`'s `globalEnv`/`env` or the cache
  goes stale silently.
- **Allocate ports centrally.** A `ports` map in `@repo/env` (or the template's
  Service Ports table) beats each service picking its own — collisions are guesswork
  to debug.
- **Docker builds from the monorepo root, layer-cached.** Use `turbo prune` to emit a
  minimal subset, then COPY manifests before source so `bun install` caches:
  ```dockerfile
  # stage 1: prune to just what @repo/api needs
  FROM oven/bun:1 AS pruner
  WORKDIR /app
  COPY . .
  RUN bunx turbo prune @repo/api --docker      # → out/json (manifests), out/full (source)

  # stage 2: install against the pruned manifests only (cached until a manifest changes)
  FROM oven/bun:1 AS installer
  WORKDIR /app
  COPY --from=pruner /app/out/json/ .
  COPY --from=pruner /app/out/bun.lock ./bun.lock
  RUN bun install --frozen-lockfile
  COPY --from=pruner /app/out/full/ .
  RUN bunx turbo run build --filter=@repo/api

  # stage 3: slim runtime
  FROM oven/bun:1-slim AS runner
  WORKDIR /app
  COPY --from=installer /app .
  CMD ["bun", "run", "apps/api/dist/index.js"]
  ```
  `turbo prune <pkg> --docker` splits output into `out/json/` (only the
  `package.json`s + lockfile, for the install layer) and `out/full/` (the source), so
  editing source does NOT bust the dependency-install layer.
- **CI runs affected packages only.** `--filter=...[origin/main]` (or the PR base)
  plus remote cache is the whole strategy — see §2 and §3.

## References

- **[references/turbo-json.md](references/turbo-json.md)** — a complete, annotated
  `turbo.json` (build/dev/test/lint/typecheck) that parses.
- **[references/diagnostics.md](references/diagnostics.md)** — graph visualization,
  cache-miss debugging (`--summarize`, `--dry=json`), circular-dependency detection,
  and shared-package impact analysis.
