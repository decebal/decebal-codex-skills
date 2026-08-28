# Annotated `turbo.json`

A complete pipeline for a Bun + Turborepo monorepo with `build`, `dev`, `test`,
`lint`, and `typecheck`. Copy it, then tune `outputs` and `env` to what your tasks
actually write and read. JSON has no comments, so the annotations follow each block;
the JSON itself is valid and parses on Turborepo 2.x.

```json
{
  "$schema": "https://turborepo.dev/schema.json",
  "ui": "tui",
  "globalDependencies": ["tsconfig.base.json", ".env"],
  "globalEnv": ["NODE_ENV", "CI"],
  "globalPassThroughEnv": ["HOME", "PATH"],
  "tasks": {
    "build": {
      "dependsOn": ["^build"],
      "outputs": ["dist/**", ".next/**", "!.next/cache/**"],
      "env": ["NEXT_PUBLIC_API_URL"]
    },
    "dev": {
      "dependsOn": ["^build"],
      "cache": false,
      "persistent": true
    },
    "test": {
      "dependsOn": ["^build"],
      "outputs": ["coverage/**"],
      "inputs": ["src/**", "test/**", "package.json"]
    },
    "lint": {
      "dependsOn": ["^build"],
      "outputs": []
    },
    "typecheck": {
      "dependsOn": ["^build"],
      "outputs": [".tsbuildinfo", "*.tsbuildinfo"]
    }
  }
}
```

## Why each line is the way it is

**Top level**

- `$schema` — points editors at the JSON schema for autocomplete and validation on
  every key below. Cheap, always include it.
- `ui: "tui"` — the interactive terminal UI (per-task panes). Use `"stream"` for CI
  or non-TTY logs.
- `globalDependencies` — files that, when changed, bust the hash of **every** task.
  Put root config that all packages read here (`tsconfig.base.json`, a root `.env`).
  A source glob does NOT belong here — that is what per-task `inputs` are for.
- `globalEnv` — env vars that impact **every** task's hash. Change `NODE_ENV` and the
  whole graph re-runs, correctly.
- `globalPassThroughEnv` — vars every task's runtime may read but that must NOT enter
  the hash (`HOME`, `PATH`). Without an allowlist, `envMode: "strict"` (the 2.x
  default) hides all non-declared vars from tasks.

**`build`**

- `dependsOn: ["^build"]` — the caret means "build every upstream **dependency**
  first." This is what makes topological ordering work: `@repo/web` won't build until
  `@repo/ui` (which it imports) has.
- `outputs` — exactly what a build writes. `dist/**` for a library, `.next/**` for a
  Next.js app, and `!.next/cache/**` to **exclude** the framework's own local cache,
  which is machine-specific and would poison a remote-cache restore. Getting this
  glob wrong is the #1 cache defect: too narrow → files missing after a hit; too broad
  → junk cached.
- `env: ["NEXT_PUBLIC_API_URL"]` — a build-time-inlined var. If it is not declared,
  two builds with different API URLs share a cache entry and ship the wrong URL.

**`dev`**

- `cache: false` — a dev server produces no reproducible artifact; caching it is
  meaningless.
- `persistent: true` — marks it long-running so Turbo (a) keeps it in its own pane
  and (b) refuses to let any one-shot task `dependsOn` it. `dependsOn: ["^build"]`
  still builds upstream libs once before the server starts.

**`test`**

- `outputs: ["coverage/**"]` — cache the coverage report so a re-run on unchanged code
  replays it instantly.
- `inputs` — narrows the change-detection to source and test files plus the manifest,
  so editing a README does not invalidate the test cache.

**`lint`**

- `outputs: []` — lint writes nothing, but it IS worth caching the *result* (a
  pass/fail + logs). An empty `outputs` array means "cacheable, no files to restore" —
  a cache hit skips re-linting unchanged code. This is very different from
  `cache: false`.

**`typecheck`**

- `outputs` includes the TypeScript incremental build info so an unchanged package
  restores it instead of re-checking. Requires `"incremental": true` +
  `"tsBuildInfoFile"` (or `composite`) in the package's `tsconfig.json`.

## Per-package overrides

A package can override a root task in its own `turbo.json` with an `extends`:

```json
{
  "extends": ["//"],
  "tasks": {
    "build": { "outputs": ["build/**"] }
  }
}
```

`"extends": ["//"]` inherits from the root config; the `build` entry replaces only
that one task's config for this package (e.g. a package that emits to `build/` instead
of `dist/`).
