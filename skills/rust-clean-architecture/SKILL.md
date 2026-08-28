---
name: rust-clean-architecture
description: "Apply clean-architecture layering to Rust workspaces and enforce dependency direction with tests. Use when placing new files, modules, or crates; reorganizing layers; splitting large modules; setting visibility; or reviewing hexagonal and ports-and-adapters boundaries."
---

# Rust Clean Architecture

Keep a Rust workspace's dependency graph pointing one way and make that direction a
compiler/test failure, not a code-review opinion. This is the actionable checklist;
the incident-backed reasoning (why ceilings beat bans, how a god module grew 54%
while docs called the fix unstarted) lives in
[../../rules/layer-boundaries.md](../../rules/layer-boundaries.md) — read it for depth.

## When to Use

- **Creating a new file, module, or crate** — decide its layer before writing code.
- **Reorganizing modules** or splitting a file that has grown past its budget.
- **Architecture review** — auditing imports, visibility, and adapter placement.

## The dependency direction

```
presentation → application → infrastructure ← domain
 (handlers,      (services,    (persistence,     (entities,
  CLI, HTTP)      use cases)    external I/O)      pure logic)
```

Imports only ever point **inward/downward**. Domain imports nothing from the
workspace. Application orchestrates. Infrastructure adapts I/O. Presentation is the
transport edge.

## Layer-placement flowchart

For a new type or file, ask in order and stop at the first yes:

1. **Is it a pure entity, value type, or business rule with no I/O and no
   framework?** → `domain`. (`struct Job`, `enum RenameProfile`, a validation fn.)
2. **Does it define a *port* — a trait the app needs but whose implementation is
   I/O?** → `domain` (the trait) . The trait lives inward; the impl lives in
   `infrastructure`. (`trait JobStore`, `trait Clock`.)
3. **Does it orchestrate a use case — coordinating domain types and ports to get
   work done, but touching no socket/file/DB/HTTP directly?** → `application`.
   (`fn suggest_connections_for_jobs(jobs: &[Job], store: &dyn JobStore)`.)
4. **Does it actually perform I/O — DB, filesystem, network, clock, env, a vendor
   SDK?** → `infrastructure`. It *implements* a domain port.
5. **Is it a transport edge — CLI arg parsing, an HTTP handler, an IPC dispatcher, a
   TUI view?** → `presentation`. It calls application, never infrastructure.

If two answers seem to fit, place it in the **innermost** layer that fits (domain
before application before infrastructure) and pass the outer data in as a parameter.

## Visibility rules

Visibility is the first, cheapest layer gate — an item the next layer can't name
can't create a bad dependency.

- **`pub(crate)` by default.** Most items are internal wiring. Start restrictive.
- **`pub` only at a crate's real API surface** — the handful of types/fns other
  crates legitimately call. Everything else stays `pub(crate)` or narrower
  (`pub(super)` for a submodule helper).
- **A field is `pub` only if callers must read/write it directly.** Prefer a
  constructor + accessors so invariants stay enforceable.
- **Re-export the API surface from the crate root**, so consumers write
  `use my_domain::Job;` not `use my_domain::entities::job::Job;`. The module path is
  then free to change without breaking callers.

```rust
// domain crate root — lib.rs
mod entities;      // private module tree
mod ports;
pub use entities::{Job, JobId};   // canonical, stable surface
pub use ports::JobStore;          // the port trait, re-exported
```

## Canonical import paths — never import through a re-export chain

A type has exactly one **canonical** home (where it is defined) and one public
surface (where its owning crate re-exports it). Import from one of those two — never
launder it through a third crate that happened to re-export it.

- **A type needed in multiple layers?** Define it in `domain`, re-export it
  **downward**. Application and presentation import it from the domain crate, not
  from each other.
- **Never re-export upward** to dodge the direction — `infrastructure` re-exporting
  an `application` type so `presentation` can grab it is the violation wearing a
  disguise.
- Bad: `use crate::presentation::handlers::Job;` (Job is a domain type; this couples
  presentation into the import path). Good: `use my_domain::Job;`.

## Adapter pattern for workspace crates

Infrastructure adapts I/O **behind a domain-defined trait (port)**. The dependency
inverts: infrastructure depends on the domain's trait, not the reverse.

```rust
// domain: the PORT (a trait) + the data it speaks
pub struct Job { /* … */ }
pub trait JobStore {
    fn load(&self, id: JobId) -> Result<Job, StoreError>;
    fn save(&self, job: &Job) -> Result<(), StoreError>;
}

// infrastructure: the ADAPTER — implements the port, owns the sqlx/reqwest/fs code
pub struct SqliteJobStore { pool: SqlitePool }
impl JobStore for SqliteJobStore { /* real I/O here */ }

// application: depends on the PORT, never on SqliteJobStore
pub fn archive_stale(store: &dyn JobStore, now: Instant) -> Result<usize, StoreError> {
    // orchestration only
}
```

Rules that keep the adapter honest:

- **Pass data as parameters, never import upward.** If infrastructure needs
  application data, the function takes it: `fn index(jobs: &[Job])`, not
  `use crate::application::…`.
- **Application depends on the trait, not the concrete adapter** — inject
  `&dyn JobStore` (or a generic `S: JobStore`). This is what makes the use case
  unit-testable with an in-memory fake.
- **The port lives with the domain; the adapter lives in infrastructure.** Test the
  adapter against a fake or in-memory transport, not a live socket (see
  [../../rules/testing-gates.md](../../rules/testing-gates.md)).

## Make the direction a TEST, not a convention

A small architecture test suite that greps imports catches every violation on the
first push. In a Rust workspace this is a `#[test]` (or an integration test under
`tests/`) that scans source files for forbidden `use`/`crate::` paths and asserts a
count.

**Ceilings, not bans, for what you cannot fix today.** Assert "at most N
violations". The number only ever ratchets down; every new violation fails the
build without demanding the whole migration land first.

Concrete Rust test names to copy:

```rust
#[test] fn domain_layer_must_not_import_upper_layers() { assert_no_imports("crates/domain/src", &["application", "infrastructure", "presentation"]); }
#[test] fn application_layer_must_not_import_presentation() { assert_no_imports("crates/application/src", &["presentation"]); }
#[test] fn services_direct_infrastructure_imports_ceiling() { assert!(count_imports("crates/application/src", "infrastructure") <= 0); }
#[test] fn presentation_must_not_import_infrastructure_directly() { /* route via a presentation::infra facade */ }
#[test] fn persistence_layer_must_not_use_direct_file_writes() { assert_ceiling("std::fs::write|File::create", &ALLOWLIST); }
#[test] fn new_source_files_must_not_carry_inline_test_modules() { /* see testing-gates.md */ }
```

Implement with `walkdir` + a substring/regex scan over `.rs` files, or a plain
`grep` shell gate. Keep the allowlist a `const` array in the test so adding to it is
a reviewable diff.

**Facade for coupling you can't remove.** Where presentation genuinely needs
infrastructure, route it through a single `presentation::infra` module and make a
direct import a test failure. The facade's call-site count is then your migration
metric — it only goes down.

## Module size thresholds

| Lines | Action |
|---|---|
| < 500 | Ideal |
| 500–1,500 | Acceptable if single responsibility |
| 1,500–3,000 | Review: can sections be extracted? |
| > 3,000 | **Requires a split plan before adding more code** |

Gate new modules at 500 lines with a **closed allowlist** of existing offenders — a
ratchet, not a rewrite.

## Splitting a god module — find the SHAPE first

Diagnose before you reorganize; getting this wrong reorganizes files for months
while the module keeps growing.

- **A file of many functions** splits along their dependency direction — lift one
  function (or one phase) at a time into a sibling module, as an ordinary function
  taking **ordinary parameters, never `&Ctx`** (which just re-hides the coupling).
- **A file that is essentially ONE huge function/closure does not split at all until
  it has a seam.** A closure captures its environment implicitly, so nothing lifts
  out until you name what it captured. Turn the closure body into
  `run_step_loop(ctx: StepLoopCtx)`, where `StepLoopCtx` is an explicit struct
  holding exactly the captured values. Now phases can be lifted as free functions.
- **Return shapes carry control flow across the new function boundary:**
  `Phase::{Ready { .. }, Failed(Error)}` replaces a mutable out-param + early
  returns; `RetryAction::{Retry, Skip, Fail}` names a decision a `continue`/`break`
  label can't cross; `park_if_requested(..) -> bool` for the one bail-without-failure
  path.
- **Prefer extending an existing sibling** over adding a module. When nothing fits,
  say why in the new module's doc comment.
- **Stopping mid-split? Write down which blocks remain and what blocks each one** —
  that sentence saves the next session a day. Never a task-tracker id in the source;
  cite the reason inline.

## Checklist

- [ ] New file's layer decided **before** writing code (flowchart above).
- [ ] Imports point inward/downward only — no upward `use`.
- [ ] Cross-layer types defined in `domain`, re-exported downward; imported from
      their canonical/root path, never through a re-export chain.
- [ ] Items are `pub(crate)` unless they're the crate's real API surface.
- [ ] I/O sits behind a domain-defined trait (port); the adapter is in
      `infrastructure`; the use case takes `&dyn Port`.
- [ ] Infrastructure receives application data as parameters, imports nothing upward.
- [ ] An architecture test asserts each direction rule (ceilings that ratchet down).
- [ ] No module over 500 lines without a single responsibility; nothing over 3,000
      without a written split plan.
- [ ] A god module was split by finding its SHAPE (functions vs one closure), not by
      moving lines blindly.
