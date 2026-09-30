# Agent parallelism — measurements and details

Detailed measurements supporting the core rules in `rules/agent-parallelism.md`.

## The file-count split decision

2026-08-07, five slices of one feature: ~5s per tool round-trip, and the machine sat idle — no compiler, no bundler, no browser running — for most of every run. A 6-file slice and a 173-file slice each took ~33 min: the small one still paid the fixed read-context / verify / report overhead, the large one paid ~5s *per file* on top. Sequential 5 slices ≈ 2.5–3h; the same work as 3 concurrent agents ran in roughly one slice's time.

## CPU contention

2026-08-11: with three competing workloads a test suite could not finish inside its 300s gate cap, and load peaked at **136** on twelve cores; alone on an idle box the same suite ran in **22 seconds**. 

Earlier: ten worktrees compiling on twelve cores gave each ~1.2 cores and made 300s unreachable **for every session at once**, so pushes failed for reasons that had nothing to do with the code being pushed.

## Process counting pitfall

`ps -eo args | grep <compiler> | grep -oE '<path>'` overstates by ~25×, because the worktree path appears once per include/link flag on every compiler command line. One session reported "768 compiler processes", told the user other agents were melting the box, and killed its own workflow to relieve load — the real count was **16**, and the load was mostly the browser. Use `pgrep -x <name> | wc -l`.

## Cold build cost

A cold compile-only test build took **60m24s** *with* a compiler cache at a 79% hit rate. The cache survives an interruption; wall clock does not, so a `timeout` wrapper or a `pkill` to free CPU restarts the clock and no gate ever becomes warm.

Measured on one project: cold lint blew the ceiling and was killed; after cloning the build dir the same command finished in **3m40s, exit 0, while another session saturated the CPU**.

## Disk cleanup and clonefiles

Six worktrees once left a machine at **1.4 GiB free of 926 GiB**, which broke the seeder mid-copy (`No space left on device`) — a worktree reporting itself unready for a reason unrelated to the work in it.

But a clonefile-seeded build dir SHARES most of its blocks with the tree it was cloned from. `du -sh` reports apparent size; deleting the copy frees only the blocks that actually diverged. Measured: a build dir `du` reported as **50 G** freed **2 GiB**.

On one machine: 38 idle build directories, 152 GB by `du`, 9.3 GiB free until seven of them were reclaimed. [`target-gc`](https://github.com/decebal/target-gc) evicts them hourly — cargo-tagged directories only, never one whose profile lock is held, whose project is a process's cwd, or whose binary a service runs.

## Formatting check comparison

A gate that compiles nothing should invoke no build tool at all: reading the manifests directly and driving the formatter ran in **1.2s** versus **11s** for two build-tool-wrapped format gates, because the package-manager cache is ONE lock for the whole machine and per-worktree build dirs do not isolate it.
