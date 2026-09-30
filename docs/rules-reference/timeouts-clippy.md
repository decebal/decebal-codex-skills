# Timeouts — the Clippy incident

Detailed measurement and analysis supporting `rules/timeouts.md` — why asking which PHASE is the load-bearing question.

## Clippy's `nonstandard_macro_braces` check

This check verifies whether a macro used its conventional delimiters. It ran *after* macro expansion, by which point the original braces were gone — so it recovered them from hygiene data, source spans and string manipulation, calling `outer_expn_data` ~**5.9 million times** on one crate, each call taking an atomic and a lock that blocks the rest of the compiler. That was ~**25% of Clippy's entire runtime**.

**Moved to run *before* expansion**, where the braces are simply visible, the same check got ~**3,133× cheaper** in under 200 lines. Nothing about the comparison got faster. **The expensive work stopped existing.**

## The consequence: moving a phase breaks assumptions

The Clippy move broke a standing policy against pre-expansion lints, and its own lintcheck showed **2 removals and 1 changed** diagnostic. That was accepted deliberately, with the difference in verdicts known and reviewed.

**This is the critical step:** Run both placements over a real corpus and compare verdict-for-verdict before keeping the speedup. A faster check that quietly answers differently is a regression wearing a stopwatch.

## The decision sequence

When a step is slow, ask in this order:

1. **What does it need, and which phase still has it?** If a later phase rebuilds what an earlier one held, that rebuild is the entire cost.
2. **Does it need the machinery of the phase it sits in at all?** A check that only reads source text needs no compiler, no expansion, no type information. If it has a build dependency, ask whether the check needs one or merely *inherited* one from how it was packaged.
3. Only then reach for the tactics below: split, cache, parallelize, precompute.

Moving a phase can change the answers, so you **must prove that it did not** before keeping the speedup.
