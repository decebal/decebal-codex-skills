# Testing and gates

## Know what actually enforces anything

Before trusting a green badge, find out whether a check can BLOCK a merge. On a
private repo under a free org plan, protected branches and rulesets are unavailable
**by plan, not by misconfiguration** — every CI check is advisory and a human can
merge straight past a red run.

> A green CI badge can mean **"someone would have seen it"** rather than **"it could
> not have merged."**

Where that is true, **the pre-push hook is not belt-and-braces — it is the
braces**, and that is why the hooks must be strict, wrapped in timeouts, and never
bypassed. `--no-verify` and skip env vars are forbidden precisely because there is
no second line behind them.

Two corollaries that have already cost real defects:

- **"Wired into CI" is not "enforced".** Wiring a gate into a workflow buys
  visibility. Still worth doing — a gate nobody can see is worse — but do not treat
  a workflow addition as closing a hole.
- **A gate that runs in neither CI nor pre-push runs nowhere at all**, and is
  indistinguishable from one that passes. This has happened more than once: an
  admin test suite that no gate invoked, and a whole backend suite skipped by the
  release workflow on the strength of a green quality workflow that ran zero
  backend tests.

## Tests that cannot fail

The worst test is not a failing one. Watch for:

- a suite whose filter matches nothing (it "passes" instantly),
- a test hitting a dev server someone else happens to be running,
- a piped command whose exit code belongs to the pipe,
- a mock so complete the assertion can only be true.

Each is an absence wearing a green badge — see
[evidence-discipline.md](evidence-discipline.md).

## On a gate failure, go GRANULAR — never re-run the same wide gate

Gates run in sequence, so **the first one to fail hides every gate behind it.**

**Never conclude "my change is clean" from a TIMEOUT.** It is the weakest signal
there is. And never answer a gate failure by re-running the same wide gate — you
learn one gate per attempt, each costing a full cycle.

Instead, drop to the smallest checks that can fail, and run them **cheapest and
most-discriminating first**:

1. **The ratchets and text-scan tests** — size caps, layer boundaries, banned
   imports, id references. Seconds once warm.
2. **The scoped unit tests** for the crate or package you touched.
3. **The lint pass** for that package.
4. Only then the whole-workspace compile gates.

**A build command is not a test run.** `cargo check`, `clippy`, and any
`--no-run` invocation COMPILE without EXECUTING. The architecture ratchet and size cap are invisible to all three.

**Detail:** `~/.codex/rules-reference/testing-gates-detail.md` — the phase-ordering incident (8.2s text scan vs 300s timeouts + cold build) and why a source scan has no inherent build dependency.

## Process-per-test, sharding, and test authoring

**Detail:** `~/.codex/agent-rules/testing-authoring.md` (`codex-guard rule-scope` delivers it the first time a session touches a test file) — process-per-test isolation, CI sharding strategy, test file conventions, un-hangable test patterns, module mock factories.

See that file when working on test authoring; it arrives only once a session touches test files or gate configs.

**In brief:** running each test in its own process isolates process-global state. Shard in CI, run whole locally. No new inline `#[cfg(test)] mod tests` blocks.
