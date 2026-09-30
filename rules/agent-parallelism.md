# Agent parallelism

How to split work across concurrent agent sessions without paying more in merge
and contention than you win in latency.

## Parallelize by FILE COUNT, not by concept

**The cost of an agent run is sequential model round-trips, not compute.** Estimate in **files touched**, and shape the work around that.

**Detail:** `~/.codex/rules-reference/agent-parallelism-measurements.md` — file-count split calculations from a real five-slice feature.

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

Before starting anything that compiles — and before diagnosing a timing-out gate —
check the compiler process count and wait for zero.

**Detail:** `~/.codex/rules-reference/agent-parallelism-measurements.md` — CPU contention numbers (load 136 on 12 cores), process counting pitfalls, cold build costs (60m24s).

- **Count PROCESSES, never grep matches** — `pgrep -x <name> | wc -l`.
- **A per-worktree build dir is not the only lock.** Package-manager caches are
  usually ONE lock for the whole machine. When diagnosing a stalled command, `lsof <cache-lock-path>` names the holder.
- **Never build in the tree you are about to push from.** A concurrent build holds
  the lock, and the gate then burns its entire budget waiting.

## Seed a new worktree BEFORE any agent runs in it

A fresh worktree is missing everything gitignored, and each gap surfaces only at push time, after the work is done. Typical gaps: dependency installs, fetched binaries, prebuilt frontend `dist/`, and a warm build directory.

Write the seeding as a **script that verifies each step on disk, names anything that did not happen, and exits non-zero when the worktree is not ready.**

On APFS, seed the build directory with `cp -Rc` — a clonefile, copy-on-write, so ~12 GB costs seconds and no disk. **A cold build directory is why the first gate times out, and it is not your diff's fault.** Each worktree keeps its OWN, cloned once at creation, diverging from then on.

**Detail:** `~/.codex/rules-reference/agent-parallelism-measurements.md` — cold build cost (3m40s after cloning vs timeout before).

## ALWAYS clean up — but `du` LIES about what you get back

The moment work is pushed, remove the worktree AND its build directory. Nothing else reclaims them.

But a clonefile-seeded build dir SHARES most of its blocks with the tree it was cloned from. `du -sh` reports apparent size; deleting the copy frees only the blocks that actually diverged. **Never delete another session's build directory to free space on a `du` number.** Check `df` before and after your own cleanup.

**Detail:** `~/.codex/rules-reference/agent-parallelism-measurements.md` — the 50G→2GiB case, the 1.4GiB free incident.

Two rules that make this safe:

- **Push before you clean, and verify with `git ls-remote`** — not with the push command's exit code.
- **Never clean a worktree that is not yours.** Check `git -C <path> status --porcelain` first.

Build directories nobody pushes from are the rest of the pile (38 on one
machine, 152 GB by `du`). [`target-gc`](https://github.com/decebal/target-gc)
evicts idle Cargo-tagged ones hourly and never one in use.
