---
paths:
  - "**/*test*.{rs,ts,tsx,js}"
  - "**/tests/**"
  - "**/__tests__/**"
  - "**/__vtests__/**"
  - ".husky/**"
  - ".github/workflows/**"
  - "**/nextest.toml"
  - "**/.config/nextest.toml"
  - "**/Taskfile.yml"
  - "tooling/**/*gate*"
---

# Testing authoring — patterns and enforcement

Detailed guidance on test authoring, supported by the core rules in `rules/testing-gates.md`.

## Process-per-test isolation

Running each test in its own process structurally isolates process-global state: `$HOME` mutation, in-process singletons, projection caches, `OnceLock`-style statics. Adopting a process-per-test runner let a ~2,900-test suite drop `--test-threads=1` as a blanket requirement and run fully in parallel.

- **Keep a hang guard.** A per-test slow-timeout that flags at 60s and **TERMINATES and names** the test at 120s ends unbounded multi-minute hangs.
- **Serialize only what is genuinely machine-global** — the OS keychain is the classic one, since process-per-test cannot isolate it. Put those tests in one named group with `max-threads = 1`. Everything else runs parallel.
- **Existing `#[serial]`-style annotations are harmless** under process-per-test and keep the fallback runner working.
- **Check whether your runner runs doctests.** Many do not. If you add a runnable doctest, add an explicit step for it — otherwise it is silently skipped.

## Shard in CI, run whole locally

Compile the test binaries **once** into an archive, then fan the archive out across runners with a deterministic partition (`hash:i/N`). The aggregate check is green only if every shard is.

**A test group must never split across machines** — a group serializes only within one process-set. Run the whole group on ONE shard and EXCLUDE it from the partitions. If your config expresses that in more than one place, say so in the file: adding one group-member test then means updating all of them.

Coverage runs as a single, deliberately **unsharded** instrumented pass.

## Test file conventions

- **No new inline `#[cfg(test)] mod tests` blocks.** Put unit tests in a sibling `foo_tests.rs` or `foo/tests.rs`, integration tests in `tests/`. Grandfather the existing ones in a closed allowlist and migrate on touch.
- **Scope the pre-push run to the changed modules**; let CI run the whole suite.
- **Detect comment-only diffs and skip the compile gates** — `gates/rust/rust-effective-diff` exits 1 when a change is comment/doc/whitespace-only.

**Detail:** See `~/.codex/rules-reference/testing-gates-detail.md` for un-hangable test code patterns and module mock factories.
