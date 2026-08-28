# Agent parallelism

How to split work across concurrent agent sessions without paying more in merge
and contention than you win in latency.

## Parallelize by FILE COUNT, not by concept

**The cost of an agent run is sequential model round-trips, not compute.** Measured
across five slices of one feature: ~5s per tool round-trip, and the machine sat
idle — no compiler, no bundler, no browser running — for most of every run. A
6-file slice and a 173-file slice each took ~33 min: the small one still paid the
fixed read-context / verify / report overhead, the large one paid ~5s *per file* on
top. Estimate in **files touched**, and shape the work around that.

- **< ~40 files → one agent.** Splitting costs more in merge than it saves in
  latency.
- **> ~40 files → split by DISJOINT FILE SETS** (package, directory, layer), run
  concurrently, one worktree each. **Never split by "concept"** — two agents
  editing the same file trade latency for conflict resolution, and that lands on
  the orchestrator.
- **A 200+ file mechanical sweep is 4+ agents**, one per package. One agent on 230
  files is ~40 min of pure edit latency before a single test runs. The same work
  as 3 concurrent agents ran in roughly one slice's time.

### Mechanics that work

- One **detached** worktree per agent (`git worktree add --detach <path> <tip>`)
  with its own dependency install — separate module dir and separate build dir, so
  no build-lock contention and no branch created.
- Each agent ends with **one commit on its detached HEAD** — no branch, no push.
  The orchestrator cherry-picks the SHAs and resolves conflicts once.
- Tell every agent **explicitly which files it owns and which its siblings own**.
  Ownership is the contract; it is what makes the merge survivable. Ask each to
  report every file it touched outside its lane — that list is the conflict
  watch-list.
- **Re-run the full gates on the COMBINED tree.** Three separately-green slices can
  still be red together (a font-metric change and a colour change landing on the
  same cards).
- **A purely mechanical transform with no judgement is a script, not an agent.**
  Use an agent when each site needs classifying (is this literal a bug, a
  deliberate palette, or a stale fallback?).
- **Never queue a user's request behind a running agent.** A two-minute git
  operation made to wait 35 minutes on an unrelated agent is the orchestrator's
  error, not the agent's. Agent runs are background work; the user is not.
- **A long agent run can outlive its own branch.** If a PR merges while follow-up
  slices are still running, the branch dies underneath them and the pre-push guard
  blocks the push. For multi-slice work, either hold the merge until every slice is
  in, or give each slice-group its own PR.
- **Decide verification depth up front and state the trade.** The screenshot /
  both-theme / built-artifact tail is roughly a third of each run. It catches
  defects that stay invisible until a user toggles — and it is separable. Offer the
  choice before starting, not after the complaint.

## Compute-bound work is the OPPOSITE of this

Latency-bound agent work parallelizes. Compiles do not.

**NEVER run two builds at once. Not per-worktree, not "a workflow compiling while a
push runs its gates". ONE build on the machine at a time, and wait.** A worktree
removes the LOCK contention, not the CPU contention, so concurrency here is not a
speed/safety trade — it is slower in wall clock AND it fails gates.

Measured: with three competing workloads a test suite could not finish inside its
300s gate cap, and load peaked at **136 on twelve cores**; alone on an idle box the
same suite ran in **22 seconds**. Earlier: ten worktrees compiling on twelve cores
gave each ~1.2 cores and made 300s unreachable **for every session at once**, so
pushes failed for reasons that had nothing to do with the code being pushed.

Before starting anything that compiles — and before diagnosing a timing-out gate —
check the compiler process count and wait for zero.

- **Count PROCESSES, never grep matches.** `ps -eo args | grep <compiler> | grep -oE '<path>'`
  overstates by ~25×, because the worktree path appears once per include/link flag
  on every compiler command line. Using it, a session reported "768 compiler
  processes", told the user other agents were melting the box, and killed its own
  workflow to relieve load — the real count was **16**, and the load was mostly the
  browser. Use `pgrep -x <name> | wc -l`.
- **A per-worktree build dir is not the only lock.** Package-manager caches are
  usually ONE lock for the whole machine, and per-worktree build dirs do not
  isolate them. A gate that compiles nothing should therefore invoke no build tool
  at all: reading the manifests yourself and driving the formatter directly ran in
  1.2s versus 11s for the two build-tool-wrapped format gates it replaced. When
  diagnosing a stalled command, `lsof <cache-lock-path>` names the holder.
- **Never build in the tree you are about to push from.** A concurrent build holds
  the lock, and the gate then burns its entire budget waiting rather than
  compiling — it reports TIMEOUT, which reads as a hang and is self-inflicted
  contention. Verify first, let it finish, then push.
- **A cold worktree's first push needs an UNINTERRUPTED warm run.** Measured: a
  cold compile-only test build took **60m24s** *with* a compiler cache at a 79% hit
  rate. The cache survives an interruption; wall clock does not, so a `timeout`
  wrapper or a `pkill` to free CPU restarts the clock and no gate ever becomes
  warm. Warm the gate's exact command, then leave it alone.

## Seed a new worktree BEFORE any agent runs in it

A fresh worktree is missing everything gitignored, and each gap surfaces only at
push time, after the work is done. Typical gaps: dependency installs (including
sub-projects that are NOT workspace members and need their own install), fetched
binaries, prebuilt frontend `dist/` a backend build script consumes, and a warm
build directory.

Write the seeding as a **script that verifies each step on disk, names anything
that did not happen, and exits non-zero when the worktree is not ready** — so a
worktree reporting success really is seeded. A seeder that copied one directory
while printing a cheerful summary produced three worktrees with no dependencies at
all; that is why the steps are checked rather than announced.

On APFS, seed the build directory with `cp -Rc` — a clonefile, copy-on-write, so
~12 GB costs seconds and no disk. **A cold build directory is why the first gate
times out, and it is not your diff's fault:** a fresh worktree compiles the whole
graph the first time anything builds in it, and the first thing that does is
usually the capped lint gate, which is therefore killed before it evaluates a
single line you wrote. Measured: cold lint blew the ceiling and was killed; after
cloning the build dir the same command finished in **3m40s, exit 0, while another
session saturated the CPU**.

This is NOT a shared build directory — that only *moves* the single lock instead of
removing it. Each worktree keeps its OWN, cloned once at creation, diverging from
then on.

## ALWAYS clean up — but `du` LIES about what you get back

The moment work is pushed, remove the worktree AND its build directory. Nothing
else reclaims them. Disk pressure is real: six worktrees once left a machine at
**1.4 GiB free of 926 GiB**, which broke the seeder mid-copy (`No space left on
device`) — a worktree reporting itself unready for a reason unrelated to the work
in it.

But a clonefile-seeded build dir SHARES most of its blocks with the tree it was
cloned from. `du -sh` reports apparent size; deleting the copy frees only the
blocks that actually diverged. Measured: a build dir `du` reported as **50 G**
freed **2 GiB**.

The consequence is a rule, not a caveat: **never delete another session's build
directory to free space on a `du` number.** You force them into a cold rebuild —
the exact gate timeout above — in exchange for possibly single-digit GB. Check
`df` before and after your own cleanup; if the number barely moved, the rest are
clones too and deleting more will not help.

```bash
df -h /                                             # the only honest number
git worktree list                                   # what exists, and where
rm -rf <path>/target && git worktree remove <path>  # build dir first
git worktree prune                                  # drop stale admin entries
df -h /                                             # what you ACTUALLY got back
```

Two rules that make this safe, both learned the hard way in one session:

- **Push before you clean, and verify with `git ls-remote`** — not with the push
  command's exit code (see [evidence-discipline.md](evidence-discipline.md)). A
  worktree deleted while its fix was still uncommitted took the fix with it, and
  the work had to be redone from scratch.
- **Never clean a worktree that is not yours.** A process check tells you whether
  *a* build is running, not whose. Check `git -C <path> status --porcelain` and
  `git -C <path> log --oneline -1` first: uncommitted changes or an unpushed commit
  mean someone is mid-flight, and removing it destroys their work.
