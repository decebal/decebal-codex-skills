---
name: rust-quality
description: "Set up workspace-level Rust quality gates with Clippy, rustdoc, cargo-sort, and unsafe-code policy. Use when starting or hardening a Cargo workspace, configuring workspace lints, wiring CI, denying broken documentation links, or reducing noisy pedantic lint output."
---

# Workspace-Level Rust Lints

Configure lints ONCE at the workspace root so every crate inherits the same gate. This is the single most effective quality lever in a Cargo workspace: one table, enforced everywhere, no per-crate drift.

## When to Use

- Bootstrapping a new Cargo workspace and deciding the lint policy up front.
- A workspace where each crate has its own ad-hoc `#![warn(...)]` attributes that have drifted apart.
- Turning on `clippy::pedantic` without drowning in false-positive noise.
- Wiring clippy + rustdoc into a pre-push gate or CI (keep each step under the 5-minute ceiling — see [../../rules/timeouts.md](../../rules/timeouts.md)).

## The policy, in three tables

Put these at the **workspace root `Cargo.toml`**. Lint groups (`all`, `pedantic`) MUST carry `priority = -1` so the specific `allow`s below them win — cargo emits lints low-priority-first, and a specific lint at the default priority `0` then overrides its group. Omit the priority and the allow-list silently does nothing.

```toml
# --- root Cargo.toml ---
[workspace.lints.clippy]
all      = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }

# Battle-tested allow-list — see references/clippy-allow-list.md for the rationale table.
module_name_repetitions            = "allow"
must_use_candidate                 = "allow"
missing_errors_doc                 = "allow"
missing_panics_doc                 = "allow"
cast_precision_loss                = "allow"
cast_possible_truncation           = "allow"
cast_sign_loss                     = "allow"
cast_possible_wrap                 = "allow"
similar_names                      = "allow"
too_many_lines                     = "allow"
doc_markdown                       = "allow"
struct_excessive_bools             = "allow"
fn_params_excessive_bools          = "allow"
match_same_arms                    = "allow"
redundant_closure_for_method_calls = "allow"
unused_self                        = "allow"
return_self_not_must_use           = "allow"
items_after_statements             = "allow"
single_match_else                  = "allow"
map_unwrap_or                      = "allow"
needless_pass_by_value             = "allow"
wildcard_imports                   = "allow"
if_not_else                        = "allow"
unreadable_literal                 = "allow"

[workspace.lints.rust]
unsafe_code = "deny"
# missing_debug_implementations = "warn"   # enable per the heuristic below

[workspace.lints.rustdoc]
broken_intra_doc_links = "deny"
```

Then **every member crate opts in** with a two-line stanza — nothing else per crate:

```toml
# --- each member crate's Cargo.toml ---
[lints]
workspace = true
```

A crate WITHOUT `[lints] workspace = true` inherits nothing. Grep for its absence as part of the checklist below.

## `unsafe_code = "deny"` and the escape hatch

Denying unsafe workspace-wide makes every `unsafe` block a compile error, which is what you want as the default. For the rare justified case, allow it on the **narrowest item possible** and pair it with a `// SAFETY:` comment explaining the invariant:

```rust
/// Reads a `u32` from the device MMIO register.
#[allow(unsafe_code)]
fn read_status(reg: *const u32) -> u32 {
    // SAFETY: `reg` is a valid, aligned, 4-byte MMIO mapping owned for the
    // lifetime of this call; the device guarantees the read has no side effects.
    unsafe { reg.read_volatile() }
}
```

Prefer `#[allow(unsafe_code)]` on a single `fn` over a whole `mod`, and never at crate level unless the entire crate is an FFI shim. The allow's blast radius is your audit surface — keep it one item wide.

## Third-party macros that generate lint-violating code

Some proc-macros expand to code you did not write and cannot annotate inline — rustler's `#[nif]`, prost/tonic generated modules, `wasm-bindgen`, etc. These commonly trip `unsafe_code`, `clippy::missing_safety_doc`, or `clippy::not_unsafe_ptr_arg_deref`. Suppress with a **crate-level `#![allow(...)]` scoped as narrowly as the crate boundary allows**, and document WHY:

```rust
// lib.rs of the NIF crate only — rustler's #[nif] macro expands to unsafe
// extern "C" fns; the generated bodies cannot carry inline SAFETY comments.
#![allow(unsafe_code)]
#![allow(clippy::missing_safety_doc)]
```

Put the generated-code crate in its own workspace member so the blanket allow does not cover hand-written code. Never widen the workspace policy to accommodate one macro — isolate the macro instead. (Keeping generated deps in a leaf crate also helps dependency hygiene — see [../../rules/dependency-hygiene.md](../../rules/dependency-hygiene.md).)

## `missing_debug_implementations`: enable or defer?

This rustc lint fires on every public type without a `Debug` impl. Whether it earns its keep depends on the crate:

| Enable it when | Defer it when |
|---|---|
| The crate is a **public library** whose types cross an API boundary — consumers expect `{:?}` to work. | The crate is a binary / internal app where types are never externally inspected. |
| Types are plain data (structs of primitives, enums) where `#[derive(Debug)]` is free. | Types hold many `Arc<dyn Trait>`, closures, or channel handles where `Debug` is `"<opaque>"` noise and hand-written impls are pure toil. |

Heuristic: **if `#[derive(Debug)]` would compile on ~all your public types, enable it; if half of them would force a hand-rolled placeholder impl, the lint is measuring the wrong thing — leave it off.** When enabling, do it as `missing_debug_implementations = "warn"` in `[workspace.lints.rust]` and burn down the warnings before promoting to `"deny"`.

## Keep manifests sorted

Wire `cargo sort` (from the `cargo-sort` crate) into a make target / CI step so dependency tables stay alphabetized and diffs stay small:

```bash
cargo install cargo-sort          # one-time
cargo sort --workspace            # rewrites every member Cargo.toml, sorted
cargo sort --workspace --check    # CI: non-zero exit if anything is out of order
```

```makefile
# Makefile
.PHONY: fmt-manifests lint
fmt-manifests:
	cargo sort --workspace

lint:
	cargo sort --workspace --check
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	cargo doc  --workspace --no-deps --all-features
```

`-D warnings` on clippy promotes the whole warn-level policy to hard errors in CI while keeping local `cargo build` warnings non-fatal.

## Setup checklist

1. Add the three `[workspace.lints.*]` tables to the **root** `Cargo.toml` (copy the snippet above).
2. Confirm every group carries `priority = -1`; a specific `allow` under a priority-0 group is a no-op.
3. Add `[lints]\nworkspace = true` to **every** member crate. Verify none was missed.
4. Run `cargo clippy --workspace --all-targets --all-features -- -D warnings` and drive it to zero.
5. Audit each surviving `#[allow(...)]`: is it the narrowest item, and does an `unsafe` allow have a `// SAFETY:` comment?
6. Isolate any generated-code crate behind its own member with a documented crate-level allow.
7. Decide `missing_debug_implementations` per the heuristic; if enabling, land it as `"warn"` first.
8. `cargo install cargo-sort`; add `cargo sort --workspace --check` and the clippy/doc steps to your Makefile/CI.
9. Confirm the whole lint gate finishes well under 5 minutes; scope the pre-push run to changed crates and let CI run the full workspace.

## References

- [references/clippy-allow-list.md](references/clippy-allow-list.md) — the 24 allowed pedantic lints, each with its allow rationale.
