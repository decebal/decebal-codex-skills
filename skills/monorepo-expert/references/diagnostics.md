# Turborepo Diagnostics

Read the artifact, not a guess — every command here makes Turbo *show* you the graph,
the hash, or the blast radius instead of you reasoning about it. See
[../../../rules/evidence-discipline.md](../../../rules/evidence-discipline.md).

## Visualize the task/dependency graph

```bash
bunx turbo run build --graph                     # graphviz DOT to stdout
bunx turbo run build --graph=graph.html          # self-contained HTML, no graphviz needed
bunx turbo run build --graph=graph.svg           # image (needs graphviz `dot` on PATH)
```

The graph shows every task node and its `dependsOn` edges (including the `^build`
topological edges). Use it to answer "why does building X also build Y?" — the edge is
right there. For image outputs install graphviz (`brew install graphviz`); the `.html`
form renders in a browser with no extra tool.

## Debug a cache MISS (or a false HIT)

A miss means an input changed; a *surprising* miss means an input you did not expect is
in the hash (usually an env var or a too-broad `inputs`/`globalDependencies`). A false
HIT means `outputs` is too narrow and a real artifact went uncached.

```bash
# What WOULD run, with each task's hash and cache status — no execution:
bunx turbo run build --dry=json | bun -e 'const j=JSON.parse(await Bun.stdin.text()); for (const t of j.tasks) console.log(t.taskId, t.cache.status, t.hash)'

# Full machine-readable run summary written to .turbo/runs/<id>.json:
bunx turbo run build --summarize
```

The summary (and `--dry=json`) records, per task: `hash`, the `inputs` file list with
their hashes, the resolved `env`/`passThroughEnv`, `dependencies`, and `outputs`. To
find *why* two runs differ, diff two summaries:

```bash
bunx turbo run build --summarize                 # run A → .turbo/runs/A.json
# ...change one thing...
bunx turbo run build --summarize                 # run B → .turbo/runs/B.json
diff <(bun -e '...print A task hashes...') <(bun -e '...print B task hashes...')
```

The task whose hash changed is the one that missed; its `inputs`/`env` list tells you
which byte moved. Common culprits:

- an env var read at build time but absent from the task's `env` → **non-deterministic
  hit**, wrong value cached (add it to `env`).
- a `globalDependencies` glob matching source → the whole graph invalidates on any
  edit (narrow it; source belongs in per-task `inputs`).
- an artifact missing after a "successful" cached build → `outputs` glob too narrow
  (widen it to everything the task writes). Verify by deleting `.turbo/cache`, running
  clean, and listing what the task produced vs what `outputs` matches.

Add `-v`, `-vv`, or `-vvv` for progressively louder logging of hash computation.

## Detect a circular dependency

Turbo refuses to run on a cyclic package graph and names the cycle:

```
error: Invalid package dependency graph: Cyclic dependency detected:
  @repo/a depends on @repo/b, @repo/b depends on @repo/a
```

If a `turbo run` bails with that, break the cycle by extracting the shared code into a
third leaf package both depend on — do NOT paper over it. `--graph` (above) draws the
loop so you can see which edge to cut. Cross-package cycles are the workspace analog of
the module-cycle problem in
[../../../rules/layer-boundaries.md](../../../rules/layer-boundaries.md).

## Impact analysis: what does a shared-package change touch?

Before editing `@repo/ui` or `@repo/env`, see the blast radius — every package that
would rebuild/retest:

```bash
# Everything affected by changes since the last commit, dry-run (nothing executes):
bunx turbo run build test --filter=...[HEAD^1] --dry=json \
  | bun -e 'const j=JSON.parse(await Bun.stdin.text()); console.log([...new Set(j.tasks.map(t=>t.package))].sort().join("\n"))'

# Human-readable version of the same list:
bunx turbo run build --filter=...[HEAD^1] --dry-run
```

`--dry-run` prints the "Packages in Scope" and per-task "Task / Hash / Cache Status"
tables without running anything — the exact set CI will execute. The leading `...`
includes **dependents**, so a change to a low-level shared package correctly shows
every downstream app it can break. If that list is larger than you expected, the shared
package is doing too much — a signal to split it, not to widen the filter.
