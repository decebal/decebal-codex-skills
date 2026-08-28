---
name: systematic-debugging
description: Enforce a structured diagnostic workflow before any fix — observe, hypothesize, test, diagnose, then fix and verify. Use when the user says "debug this", "why is this failing", "investigate the error", "find the bug", "troubleshoot", or "diagnose", or reports something broken, throwing, failing, or behaving unexpectedly. Covers a two-tier triage-then-deep-dive model and links reference guides for logs, network, state, and environment.
---

# Systematic Debugging

A bug is diagnosed, not guessed. This skill enforces one rule above all others:
**never propose a fix before you have named the root cause.** The most expensive
failure mode in debugging is theorizing for an hour with no evidence, then layering
guess-fixes on top of each other until the five-minute regression is a four-hour
mess.

Mirror the repo debugging rules while you work:
- [../../rules/debugging-discipline.md](../../rules/debugging-discipline.md) —
  instrument before theorizing; revert before layering; identify the regression commit.
- [../../rules/evidence-discipline.md](../../rules/evidence-discipline.md) — look at
  the artifact, not a number reporting on it; read runtime state, never guess it.

## When to Use

Triggers: "debug this", "why is this failing", "investigate the error", "find the
bug", "troubleshoot", "diagnose", or any report of a crash, exception, failing test,
wrong output, or unexpected behavior.

## Start with two-tier triage

Do NOT jump straight to deep trace analysis — it burns the context window reading
detail you may not need. Escalate only when the cheap tier points at a problem.

**TIER 1 — quick triage (always start here).** Cheap, broad signals that localize
the failure in seconds:
- Is the thing even up? Health endpoint, process running (`pgrep -x <name>`), port
  listening, last deploy status.
- What is the obvious signal? The actual error text, the HTTP status class, the exit
  code, the most recent log line, a red test name.
- Did it work before? `git log --oneline -10` on the surface, recent deploys, config
  changes.

If Tier 1 gives you the answer (service down, credential expired, obvious typo in
the last commit) — stop, you are done. Go to FIX.

**TIER 2 — deep dive (only after triage points somewhere).** Expensive, narrow
work: full trace/log correlation, adding instrumentation, stepping through state,
bisecting commits. Enter this tier pointed at a specific layer, never blind.

## The diagnostic workflow

Run these phases in order. Do not skip to FIX.

### 1. OBSERVE — reproduce and gather symptoms
- **Read the ACTUAL error.** The real message, stack trace, status code — not what
  you infer from a function's name. A name tells you intent; the error tells you what
  happened. See evidence-discipline: a claim *about* the artifact is not the artifact.
- Reproduce it. A bug you cannot reproduce is a bug you cannot confirm fixed. Capture
  the exact inputs, flags, and environment that trigger it.
- Record the symptom in one sentence, symptom first — that sentence is what the next
  session will search for.

### 2. HYPOTHESIZE — 2–3 competing explanations
- Write down two or three distinct causes that could produce this symptom. One
  hypothesis is a guess; competing hypotheses force a discriminating test.
- **Isolate the layer** each hypothesis lives in: data / logic / I/O / config /
  environment. Naming the layer tells you which reference guide to open:
  - logs and error output → [references/log-analysis.md](references/log-analysis.md)
  - HTTP / WebSocket / gRPC / connection failures → [references/network-debugging.md](references/network-debugging.md)
  - stale frontend state, DB reads, cache → [references/state-debugging.md](references/state-debugging.md)
  - env vars, secrets, TLS, DNS, PATH → [references/environment-debugging.md](references/environment-debugging.md)

### 3. TEST — the minimal experiment that discriminates
- Design the smallest experiment that RULES OUT at least one hypothesis. A test that
  every hypothesis predicts the same result for tells you nothing.
- **Change one variable at a time.** If you change two, a passing result does not tell
  you which one mattered.
- **Instrument before theorizing.** If you cannot see the event chain, add the log
  line FIRST (flags, target, payload, timestamps), run it, and READ it — do not reason
  about what the log *would* say.
- **Confirm which condition failed, not which condition exists.** A refusal covering
  `allowed = feature_on && domain_permitted` tells you nothing about which conjunct was
  false. Read the runtime state for each.

### 4. DIAGNOSE — name the root cause, not the symptom
- State the single cause the evidence supports. "The retry loop re-creates the client
  each iteration, dropping the connection pool" is a root cause. "Requests are slow"
  is a symptom.
- If the evidence does not yet single out one hypothesis, you are still in TEST —
  go back. **You may not proceed to FIX without a named root cause.**
- **Check recent changes.** If it worked before: `git log --oneline -10`,
  `git diff HEAD~3`, then `git show <hash> -- <file>` on the commit that touched the
  surface. The regression commit's diff usually contains the smoking gun.

### 5. FIX — the minimal correct change
- Fix the root cause, not the symptom. A `.to_lowercase()` masking a bad value is a
  symptom patch; the wrong value's source is the fix site.
- Smallest change that corrects the cause. No opportunistic refactors riding along.
- **Revert before layering.** If the fix does not work, `git checkout HEAD -- <file>`
  BEFORE trying the next idea. Never stack guesses.

### 6. VERIFY — confirm the fix and check for regressions
- Reproduce the original failing case and confirm it now passes — using the same
  repro from OBSERVE.
- **Verify at the destination, not via a proxy.** A green test count, an exit code, or
  a "should be fixed now" is a claim; the passing repro and the corrected artifact are
  evidence. Read the artifact.
- Run the surrounding tests / adjacent flows to confirm you did not break a neighbor.
- Add a regression test that fails without the fix and passes with it, where the
  surface supports one.

## The one-line contract

OBSERVE → HYPOTHESIZE → TEST → **DIAGNOSE** → FIX → VERIFY. No fix before DIAGNOSE.
Read the real error. One variable at a time. Revert before layering. Confirm at the
artifact.
