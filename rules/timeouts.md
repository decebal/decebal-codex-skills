# The 5-minute rule

**No shell command, gate, hook, script, or CI step may run longer than 5 minutes.
Ever.** 5 minutes is a ceiling, not a target — most steps should be seconds. If
something *needs* more than 5 minutes, that is a bug in the step, not a reason to
raise the limit. Fix the step so it fits.

## Why

A step that takes 15–20 minutes (a cold container build, a full serial test suite,
a hung network wait) blocks the human, hides its own failures behind a wall of
waiting, and piles up zombie processes that thrash shared locks.

A 5-minute ceiling forces the work to be split, cached, or parallelized until it is
fast — which is always possible.

## Enforcement

- **Every gate runs under `timeout -k 15 300`.** A hung gate is KILLED, not waited
  on. `gates/sh/hook-gate-lib.sh` provides the wrapper, the failure accounting and
  the timing output for both `pre-commit` and `pre-push`.
- **`GATE_TIMEOUT` may be LOWERED but is clamped at 300.** Raising it is the one
  wrong fix this rule exists to forbid, so the library does not honour a higher
  value. Make it explicit in code, not in a comment.
- **Any new script that shells out to a long command must wrap it in `timeout 300`**
  (`gtimeout` on macOS) and fail loudly on timeout — never silently retry or wait
  longer.
- **Commands an agent runs must be expected to finish in < 5 min.** If a command
  can't, redesign it; don't background it and babysit.

## Before making a step faster, ask which PHASE it runs in

The cheapest optimisation is never a faster algorithm. It is noticing that the
step **reconstructs information an earlier phase had for free**, and moving it
to that phase so the reconstruction stops existing.

Clippy's `nonstandard_macro_braces` checks whether a macro used its conventional
delimiters. It ran *after* macro expansion, by which point the original braces
were gone — so it recovered them from hygiene data, source spans and string
manipulation, calling `outer_expn_data` ~**5.9 million times** on one crate, each
call taking an atomic and a lock that blocks the rest of the compiler. That was
~**25% of Clippy's entire runtime**. Moved to run *before* expansion, where the
braces are simply visible, the same check got ~**3,133× cheaper** in under 200
lines.

Nothing about the comparison got faster. The expensive work stopped existing.

So when a step is slow, ask in this order:

1. **What does it need, and which phase still has it?** If a later phase rebuilds
   what an earlier one held, that rebuild is the entire cost.
2. **Does it need the machinery of the phase it sits in at all?** A check that
   only reads source text needs no compiler, no expansion, no type information.
   If it has a build dependency, ask whether the check needs one or merely
   *inherited* one from how it was packaged.
3. Only then reach for the tactics below: split, cache, parallelise, precompute.

**Moving a phase can change the answers, so prove that it did not.** The Clippy
move broke a standing policy against pre-expansion lints, and its own lintcheck
showed **2 removals and 1 changed** diagnostic. That was accepted deliberately,
with the difference in verdicts known and reviewed. Run both placements over a
real corpus and compare verdict-for-verdict before keeping the speedup: a faster
check that quietly answers differently is a regression wearing a stopwatch.

## How to fit in 5 minutes (never raise the limit)

| Slow thing | Wrong fix | Right fix |
|---|---|---|
| Cold container build for deploy | build locally and wait | fix the remote builder (usually OOM — give it memory), keep the build cache mount warm |
| Full serial test suite | run the whole thing in the hook | scope to the **changed modules** in the hook, defer the full suite to CI, which shards it |
| Long CI job | one big job | split into parallel shards, each < 5 min |
| A network wait that can hang | a longer timeout | a **shorter** timeout plus a keepalive/heartbeat so the wait has nothing to do |
| A compile gate timing out | assume a test hangs; raise the cap | Look at the **build fingerprint** first. Gates that differ in feature set, target selection, or check-vs-build mode each compile the graph from scratch — a bare type-check alone measured 350s cold. Make the gates share ONE fingerprint (build the test targets, then lint `--all-targets` with the same features, then run them) and add a compiler cache for a cold build dir |
| A compile gate timing out **when the fingerprint is already unified and the compiler cache is already on** | keep warming, keep waiting, or conclude the gate is broken | **Count the competing worktrees.** Ten worktrees on twelve cores put every session below ~1.2 cores and made 300s unreachable for all of them at once, with the cache healthy at a 79% hit rate. The fix is fewer concurrent worktrees (~`cores / 4`), not a bigger cap. See [agent-parallelism.md](agent-parallelism.md) |

## Two failure modes the wrapper also fixes

- **A hook that wraps nothing.** One repo's pre-push wrapped every gate in
  `timeout`; its pre-commit wrapped nothing, so a step that wedged there blocked
  the commit indefinitely with no diagnosis and no way out. Both hooks must use the
  same library.
- **A gate whose exit code is eaten by a pipe.** `<gate> | tail` reports the pipe's
  status. Capture the gate's own status — the library does.

## The principle

If you are about to wait more than 5 minutes for a command, STOP and make the
command faster: split, cache, parallelize, precompute. Waiting longer is never the
answer.
