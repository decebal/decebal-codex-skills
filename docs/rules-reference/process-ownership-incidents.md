# Process ownership — incidents and detection patterns

Detailed incidents and procedures supporting `rules/process-ownership.md`.

## The cost, 2026-09-18

A Mac reported as "nothing running that justifies it" was at **load average 92, peaking 251, on 12 cores, 0% idle**. Nothing in the task tracker explained it. Two independent leaks, one invariant:

| Door | Owner that died | Orphan | Cost |
|---|---|---|---|
| A browser automation daemon exits without reaping its browser | `agent-browser` daemon | 9 headless Chrome trees, **92 processes**, software-rasterizing at 100% each | **~810% CPU — 8 of 12 cores**, oldest orphan 3 days |
| A statusline shells out on a worker thread and abandons it | the statusline binary, killed by its host at its 800 ms budget | a rolling backlog of 7-second queries at 25% CPU each | load 92 → 251, re-seeded on **every render × 7 sessions** |

After killing both: **load 3.2, 83% idle.** Nothing else changed.

### The tragedy of correctness-but-not-shipped

The second one is the instructive one. Its fix had already been written — it owned the child, killed it, waited on it, and its comment documented the *same* incident from 18 days earlier: *"nine such orphans at PPID 1, the oldest 88s."* The fix was never committed, never published, never installed. The invariant was named correctly and the defect recurred anyway, because **naming it in a working tree is not shipping it**. A fix that is not installed is not a fix; verify at the destination.

## Detection: the PPID-1 sweep

An orphan is invisible to every tool that reasons about *your* jobs, so look at the process table. Read it in this order, because each answers a different question:

```bash
uptime                                   # load average vs core count
ps -Ao pid,ppid,pcpu,etime,comm -r | head -30   # who's running, sorted by CPU
ps -Ao pid,ppid,etime,comm | awk '$2==1' | wc -l   # orphan population
```

- **`load average` >> cores, but `%idle` high** → the load is uninterruptible or decaying, not CPU. Do not kill anything yet.
- **`%idle` at 0 with no build running** → something is spinning. Sort by `%CPU`.
- **`ELAPSED` in days on a process that should be per-session** → that is your orphan, regardless of what it is named.

**Count processes, never grep matches.** Under an output-filtering proxy, `ps -A | wc -l` reported **31** against a real **1242**, and truncated argv hid the flags that identified the orphans. Use the proxy's raw passthrough for every count and every argv read.

## Killing safely

Match on something only the orphan carries — its own binary name or its private profile/data directory in argv — never on the shared application name. The user's real browser and the automation's headless browser are the same executable; only argv separates them.

```bash
/usr/bin/pgrep -f <marker> | wc -l                  # 1. count
/usr/bin/pgrep -f <marker> | xargs ps -o comm= -p   # 2. READ the set before signalling
/usr/bin/pgrep -f <marker> | xargs kill -TERM       # 3. only now
```

**Never kill a build, a gate, or another session's work to reclaim CPU** — that is someone's 60-minute cold compile, and it restarts from zero. Check `git -C <path> status --porcelain` and the process's parent chain first; a live parent means someone is mid-flight.

## Enforcement protocol

A rule with no gate is advice, and this one already proved that a *correct fix in a dirty working tree* stops nothing.

- **Sweep at session start.** Count PPID-1 orphans of the binaries known to leak and surface the number. A population that only grows is the signal.
- **Ship the fix, then verify the installed artifact** — not the source. Read the binary or the installed version, because `cargo install` sees no upgrade when the version string did not change, and a published crate can be months behind the repo that fixed it.
- **A hot-path spawn is a review trigger.** Any new `Command`/`subprocess` on a render, hook, keystroke or poll path needs its budget, its cache TTL and its reap shown in the diff.
