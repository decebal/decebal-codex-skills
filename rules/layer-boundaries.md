---
paths:
  - "**/*.rs"
  - "**/Cargo.toml"
  - "**/src/**/*.{ts,tsx,svelte}"
---

# Layer boundaries and god modules

## The dependency direction

```
presentation → application → infrastructure ← domain
 (handlers)     (services)   (persistence /    (entities)
                              external I/O)
```

Domain is pure types and logic and imports nothing from the app. Application
orchestrates. Infrastructure adapts I/O. Presentation is the transport edge.

**Make the direction a test, not a convention.** A small architecture test suite
that greps imports catches every violation on the first push:

| Rule | Test |
|---|---|
| Domain must never import application, infrastructure, or presentation | `domain_layer_must_not_import_upper_layers` |
| Application must never import presentation | `application_layer_must_not_import_presentation` |
| Services must not import infrastructure directly | `services_direct_infrastructure_imports_ceiling` (must be 0) |
| No direct file writes in the persistence/services layers | `persistence_layer_must_not_use_direct_file_writes` (allowlist) |
| No untyped handler registration in the router | ceiling of 0 |
| No manual error wrapping in the router | ceiling of 0 |
| New source files must not carry inline test modules | see [testing-gates.md](testing-gates.md) |

**Ceilings, not bans, for the rules you cannot satisfy today.** A test asserting
"at most N violations" ratchets down and makes every new one a failure, without
demanding the migration land first.

**Give the layers you cannot decouple a facade.** Where presentation genuinely
needs infrastructure, route it through a single `presentation::infra` module and
make a direct import an error. The facade's call-site count is then the migration
metric.

### When adding new code

- **New module?** Decide the layer first.
- **Importing across layers?** Only downward.
- **A type needed in multiple layers?** Define it in the domain and re-export.
- **Infrastructure needs application data?** Pass it as a parameter —
  `suggest_connections_for_jobs(jobs: &[Job])`, not an import.

### Module size guidelines

| Threshold | Action |
|---|---|
| < 500 lines | Ideal |
| 500–1,500 | Acceptable if single responsibility |
| 1,500–3,000 | Review: can sections be extracted? |
| > 3,000 | Requires a split plan before adding more code |

Gate new modules at 500 lines with a **closed allowlist** for the existing
offenders. A ratchet beats a rewrite.

## Opening a god module — find the SHAPE first

A file of many functions splits along their dependency direction. **A file that is
one enormous function does not split at all until that function has a seam.** Get
this diagnosis wrong and you reorganize files for months while the module grows.

**Detail:** `~/.codex/rules-reference/layer-boundaries-godmodule.md` — the seam strategy, a real 54% growth case, return shapes that work, the cheap 969→340 example.

- **Prefer extending an existing sibling** over adding a module; most were created
  for exactly this. When nothing fits, say why in the new module's doc comment.
- **When you stop mid-split**, write down which blocks remain and what blocks each
  one, so nobody re-derives the analysis later.

## Frontend layering

```
App (composition root)
  ├── domains/{name}/{components,state,services,index.ts}
  ├── shared/{components,services,events}
  └── infrastructure/{ipc,http}
```

- **Every domain has an `index.ts` barrel.** External consumers import the barrel,
  never internal files.
- **`shared/` must not import from domains** — gate it. Keep a short, justified
  allowlist for composition chrome and app-wide wiring, and require an
  architectural justification in the script itself to add to it.
- **Domains should not import each other's internals.** A dashboard composing
  domain components is fine; reaching into another domain's component directory is
  not.
- **Navigation helpers are cross-cutting** — route through a router module, not a
  direct domain import.
