# Layer boundaries — opening a god module

Detailed guidance on the seam strategy for splitting large modules, supporting `rules/layer-boundaries.md`.

## The real case: from 2,445 to 3,762 lines

A module grew 54% while the docs described the fix as unstarted. Two wrong beliefs let this happen:

### Wrong belief 1: "The fix is trait-based DI at the entry point"

It already existed. The executor trait, its default implementation, and a host-capability DI bundle with a headless constructor were all present. DI was done. The trait even named no UI-framework type — pinned by a text-scan test *and* by a never-invoked function whose body type-checks every entry point against a headless host, so reintroducing the framework handle is a **compile error**.

### Wrong belief 2: "Splitting only works if the functions have one-directional dependencies"

There was essentially ONE function. ~3,460 of the 3,762 lines sat inside a single `move` closure passed to a spawner, and 2,862 of those were one `for` loop body. **A closure captures its environment implicitly**, so nothing could be lifted out without first discovering, by compiling, what it had captured.

## The seam strategy

**Name the closure first.** Turn its body into `run_step_loop(ctx: StepLoopCtx)`, where `StepLoopCtx` is an explicit struct holding exactly what the closure used to capture. The setup function shrinks to ~55 lines ending in `spawner.spawn_blocking(Box::new(move || run_step_loop(ctx)))`.

Then lift one PHASE at a time into a sibling module, as an ordinary function taking ordinary parameters — **never `&Ctx`, which would just re-hide the coupling.**

### Return shapes that carry seams

- **`Phase::{Ready { … }, Failed(Error)}`** — replaces a mutable out-parameter (now a field of `Ready`) and several `handle_failure(…); return;` early exits.
- **`RetryAction::{Retry, Skip, Fail(Error)}`** — a labelled `continue 'retry` / `break 'retry` cannot cross a function boundary, so the phase names the DECISION and the loop translates it back into its own control flow. The counter stays owned by the loop and is passed `&mut`.
- **`park_if_requested(…) -> bool`** — the ordinary park shape, for the one place the loop bails without a failure. `true` means parked, caller returns.

### Prefer extending a sibling

Most god modules already have several extraction-ready siblings. Prefer extending an existing one over adding a new module; when nothing fits, say why in the new module's doc comment.

## The cheap version: 969 → 340 lines

It was never one closure — it was seven ordinary functions, one of which held four phases that only ever ran in sequence. They needed somewhere honest to go, nothing more.

**Shape worth copying:** a dispatch needing **ten** values, over the linter's argument ceiling, takes an owned param struct and destructures it back into locals with the same names on line one. The body is then the original `match` verbatim — a param struct destructured immediately costs one line and keeps the moved code byte-identical, which is what makes a split reviewable.

## Recording what is still inside

When you stop mid-split, write down which blocks remain and **what blocks each one**. Example: a cost-accounting block could not move because it reads a file-private capability lookup *after* the turn rather than before, so the value cannot be passed in without changing when it is read — moving it together with the pre-step budget gate is the shape that works. That sentence saves the next session a day.
