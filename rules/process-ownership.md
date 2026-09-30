# Process ownership — every child has a parent that reaps it

Portable. No stack assumptions beyond a POSIX process table.

## The invariant

> **A process you spawn must be owned, for its whole life, by something that will
> outlive it and reap it. If the owner can die first, the child is a leak.**

When the owner dies first, the child reparents to init (PPID 1) and runs to
completion with nobody waiting on it, nobody killing it, and nobody counting it.
It does not appear in any job list, any task tracker, or any "what is running"
mental model — which is exactly why the machine looks idle while it is not.

Name the invariant in the fix. A fix that only repairs the symptom gets re-broken
through the next door, and this one has four doors below.

## The cost, 2026-09-18

A Mac reported as idle was at **load average 92, peaking 251, on 12 cores, 0% idle**. Nothing in the task tracker explained it. Two independent leaks, one invariant.

**Detail:** `~/.codex/rules-reference/process-ownership-incidents.md` — the full incident table (browser orphans, statusline storm), why the fix existed but wasn't shipped, the tragedy of correctness-without-deployment.

## Authoring: spawning a child correctly

- **Own the child, not a thread that owns the child.** A worker thread blocked in
  `output()`/`communicate()`/`wait()` cannot enforce a deadline, because the
  deadline belongs to whoever can *kill* the child. When the process exits, the
  thread dies and the child survives it. Hold the handle in the code that owns
  the budget, poll it, and kill it yourself.
- **Kill THEN wait.** `kill()` alone converts an orphan into a zombie — a
  different leak, not a fix. Always reap.
- **One deadline for the whole operation, not one per attempt.** If you retry two
  spellings, two flags or two endpoints, they share the budget the caller was
  promised. Two budgets cost twice what the constant says.
- **A spawn on a per-render / per-event / per-keystroke path is unbounded
  concurrency** unless it is *bounded, cached and reaped*. If the work takes
  longer than the interval between firings, the firings overlap and pile up
  without limit. The pile-up rate is `sessions × renders/sec`, and none of those
  terms is under the spawn site's control.
- **Cache with a TTL on those paths, and serve stale on timeout.** Stale and
  correct beats fresh and unbounded. Timing out should fall back to the last good
  value, never to another spawn.
- **Prefer reading the file to spawning the tool.** Reading `.git/HEAD` costs
  microseconds; `git rev-parse` costs a fork, an exec and a process. On a hot
  path that difference is the entire budget.

## Detection: the PPID-1 sweep

An orphan is invisible to every tool that reasons about *your* jobs.

**Detail:** `~/.codex/rules-reference/process-ownership-incidents.md` — the detection procedure, the process counting pitfall (pgrep vs grep overstating by 25×).

Read it in this order: `load average` vs cores, `%idle`, `ELAPSED` in days. Use `pgrep -x <name> | wc -l`, never grep.

## Killing safely

Match on something only the orphan carries — its own binary name or its private profile/data directory in argv — never on the shared application name.

**Never kill a build, a gate, or another session's work to reclaim CPU.** Check `git -C <path> status --porcelain` first; a live parent means someone is mid-flight.

## Enforcement

A rule with no gate is advice, and this one already proved that a *correct fix in
a dirty working tree* stops nothing. So:

- **Sweep at session start.** Count PPID-1 orphans of the binaries known to leak
  and surface the number. A population that only grows is the signal.
- **Ship the fix, then verify the installed artifact** — not the source. Read the
  binary or the installed version, because `cargo install` sees no upgrade when
  the version string did not change, and a published crate can be months behind
  the repo that fixed it.
- **A hot-path spawn is a review trigger.** Any new `Command`/`subprocess` on a
  render, hook, keystroke or poll path needs its budget, its cache TTL and its
  reap shown in the diff.
