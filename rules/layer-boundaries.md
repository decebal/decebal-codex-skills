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

A real case, and the two wrong beliefs that let it grow 54% (2,445 → 3,762 lines)
while the docs described the fix as unstarted:

- **"The fix is trait-based dependency injection at the entry point."** It already
  existed. The executor trait, its default implementation, and a host-capability DI
  bundle with a headless constructor were all present. DI was done. The trait even
  named no UI-framework type — pinned by a text-scan test *and* by a never-invoked
  function whose body type-checks every entry point against a headless host, so
  reintroducing the framework handle is a **compile error**.
- **"Splitting only works if the functions have one-directional dependencies."**
  There was essentially ONE function. ~3,460 of the 3,762 lines sat inside a single
  `move` closure passed to a spawner, and 2,862 of those were one `for` loop body.
  **A closure captures its environment implicitly**, so nothing could be lifted out
  without first discovering, by compiling, what it had captured.

### The seam

Name the closure. Turn its body into `run_step_loop(ctx: StepLoopCtx)`, where
`StepLoopCtx` is an explicit struct holding exactly what the closure used to
capture. The setup function shrinks to ~55 lines ending in
`spawner.spawn_blocking(Box::new(move || run_step_loop(ctx)))`.

Then lift one PHASE at a time into a sibling module, as an ordinary function taking
ordinary parameters — **never `&Ctx`, which would just re-hide the coupling.**

Three return shapes carry the seams:

- `Phase::{Ready { … }, Failed(Error)}` — replaces a mutable out-parameter (now a
  field of `Ready`) and several `handle_failure(…); return;` early exits.
- `RetryAction::{Retry, Skip, Fail(Error)}` — a labelled `continue 'retry` /
  `break 'retry` cannot cross a function boundary, so the phase names the DECISION
  and the loop translates it back into its own control flow. The counter stays
  owned by the loop and is passed `&mut`.
- `park_if_requested(…) -> bool` — the ordinary park shape, for the one place the
  loop bails without a failure. `true` means parked, caller returns.

**Prefer extending an existing sibling** over adding a module; most were created
for exactly this. When nothing fits, say why in the new module's doc comment.

### The cheap version of the same exercise

A 969-line review module became **340** with no `Ctx` struct at all. It was never
one closure — it was seven ordinary functions, one of which held four phases that
only ever ran in sequence. They needed somewhere honest to go, nothing more.

One shape worth copying: a dispatch needing **ten** values, over the linter's
argument ceiling, takes an owned param struct and destructures it back into locals
with the same names on line one. The body is then the original `match` verbatim — a
param struct destructured immediately costs one line and keeps the moved code
byte-identical, which is what makes a split reviewable.

### Record what is still inside, so nobody re-derives it

When you stop mid-split, write down which blocks remain and **what blocks each
one**. Example: a cost-accounting block could not move because it reads a
file-private capability lookup *after* the turn rather than before, so the value
cannot be passed in without changing when it is read — moving it together with the
pre-step budget gate is the shape that works. That sentence saves the next session
a day.

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
