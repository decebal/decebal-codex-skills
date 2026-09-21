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

A Mac reported as "nothing running that justifies it" was at **load average 92,
peaking 251, on 12 cores, 0% idle**. Nothing in the task tracker explained it.
Two independent leaks, one invariant:

| Door | Owner that died | Orphan | Cost |
|---|---|---|---|
| A browser automation daemon exits without reaping its browser | `agent-browser` daemon | 9 headless Chrome trees, **92 processes**, software-rasterizing at 100% each | **~810% CPU — 8 of 12 cores**, oldest orphan 3 days |
| A statusline shells out on a worker thread and abandons it | the statusline binary, killed by its host at its 800 ms budget | a rolling backlog of 7-second queries at 25% CPU each | load 92 → 251, re-seeded on **every render × 7 sessions** |

After killing both: **load 3.2, 83% idle.** Nothing else changed.

The second one is the instructive one. Its fix had already been written — it
owned the child, killed it, waited on it, and its comment documented the *same*
incident from 18 days earlier: *"nine such orphans at PPID 1, the oldest 88s."*
The fix was never committed, never published, never installed. The invariant was
named correctly and the defect recurred anyway, because **naming it in a working
tree is not shipping it**. A fix that is not installed is not a fix; verify at
the destination, per [evidence-discipline.md](evidence-discipline.md).

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

An orphan is invisible to every tool that reasons about *your* jobs, so look at
the process table:

```bash
uptime                                   # load average vs core count
rtk proxy ps -Ao pid,ppid,pcpu,etime,comm -r | head -30
rtk proxy ps -Ao pid,ppid,etime,comm | awk '$2==1' | wc -l   # orphan population
```

Read it in this order, because each answers a different question:

- **`load average` >> cores, but `%idle` high** → the load is uninterruptible or
  decaying, not CPU. Do not kill anything yet.
- **`%idle` at 0 with no build running** → something is spinning. Sort by `%CPU`.
- **`ELAPSED` in days on a process that should be per-session** → that is your
  orphan, regardless of what it is named.

**Count processes, never grep matches**, and never trust a filtered `ps`. Under
an output-filtering proxy, `ps -A | wc -l` reported **31** against a real
**1242**, and truncated argv hid the flags that identified the orphans. Use the
proxy's raw passthrough (`rtk proxy`) for every count and every argv read.
See [evidence-discipline.md](evidence-discipline.md).

## Killing safely

Match on something only the orphan carries — its own binary name or its private
profile/data directory in argv — never on the shared application name. The user's
real browser and the automation's headless browser are the same executable; only
argv separates them.

```bash
/usr/bin/pgrep -f <marker> | wc -l                  # 1. count
/usr/bin/pgrep -f <marker> | xargs ps -o comm= -p   # 2. READ the set before signalling
/usr/bin/pgrep -f <marker> | xargs kill -TERM       # 3. only now
```

**Never kill a build, a gate, or another session's work to reclaim CPU** — that
is someone's 60-minute cold compile, and it restarts from zero. Check
`git -C <path> status --porcelain` and the process's parent chain first; a live
parent means someone is mid-flight. See [agent-parallelism.md](agent-parallelism.md).

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
