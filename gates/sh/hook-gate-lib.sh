#!/usr/bin/env bash
#
# Shared gate machinery for pre-commit and pre-push.
#
# Both hooks are bound by the same rule — rules/timeouts.md: no step may run
# longer than 5 minutes, and 300s is a ceiling rather than a target. In the repo
# this came from, only ONE of them obeyed it. pre-push wrapped every gate in
# `timeout -k 15 $GATE_TIMEOUT`; pre-commit wrapped nothing at all, so a step
# that wedged there blocked the commit indefinitely, with no diagnosis and no way
# to tell a hang from a slow compile.
#
# Keeping ONE copy of the ceiling, the clamp, the bounded runner and the
# cold-cache probe is what stops the two hooks drifting apart again — the drift
# is what produced the gap in the first place.
#
# Sourced, never executed:
#   . "$REPO_ROOT/gates/sh/hook-gate-lib.sh"
#
# Sets:      GATE_TIMEOUT, TIMEOUT_BIN, HOOK_OUTPUT
# Provides:  hook_bounded, hook_target_is_cold, hook_warm_compile

# ── The ceiling ─────────────────────────────────────────────────────────────
# Override per invocation with GATE_TIMEOUT=<seconds>. LOWERING is honoured.
# RAISING is clamped: a step that needs more than 5 minutes is a bug in the step,
# and the ceiling is the forcing function that gets it fixed. The next person in
# a hurry should not be able to switch it off.
GATE_TIMEOUT="${GATE_TIMEOUT:-300}"
if [[ "$GATE_TIMEOUT" =~ ^[0-9]+$ ]] && [[ "$GATE_TIMEOUT" -gt 300 ]]; then
  echo "  [·] GATE_TIMEOUT=$GATE_TIMEOUT clamped to 300 — fix the gate, not the ceiling"
  GATE_TIMEOUT=300
fi
TIMEOUT_BIN="$(command -v timeout || command -v gtimeout || true)"

# ── Bounded runner ──────────────────────────────────────────────────────────
# Run CMD (a shell command string) under the ceiling.
#
# Output is CAPTURED into $HOOK_OUTPUT rather than streamed. Callers print their
# own one-line verdict, and an expected-and-handled failure — stale generated
# types, say — must not dump a multi-thousand-line diff over the commit. The
# caller decides what to show and whether the failure is fatal.
#
# Returns the command's exit status. 124 (TERM) / 137 (KILL) mean the ceiling
# killed it, which is a bug in the step and never a reason to raise the ceiling.
hook_bounded() {
  local cmd="$1" rc
  # HOOK_OUTPUT is this function's SECOND return value, read by the sourcing hook
  # after the call — shellcheck cannot see across the source boundary and reports
  # it unused, hence the disables.
  #
  # `if` guards the capture from `set -e` in a sourcing hook: a failing command
  # must come back as a return code for the caller to handle, not abort the hook
  # with the command's own status.
  if [[ -n "$TIMEOUT_BIN" ]]; then
    # shellcheck disable=SC2034
    if HOOK_OUTPUT=$("$TIMEOUT_BIN" -k 15 "$GATE_TIMEOUT" bash -c "$cmd" 2>&1); then rc=0; else rc=$?; fi
  else
    # shellcheck disable=SC2034
    if HOOK_OUTPUT=$(bash -c "$cmd" 2>&1); then rc=0; else rc=$?; fi
  fi
  return "$rc"
}

# ── One gate, with its verdict on one line ──────────────────────────────────
# The ordinary caller. Prints a one-line verdict, shows the tail of the output
# only on failure, and accumulates into $FAILED for the hook's exit status.
#
# Usage:  run_gate "[scope] what it checks" "<command>"
FAILED=0
run_gate() {
  local label="$1" cmd="$2" rc start elapsed
  start=$SECONDS
  printf '  [·] %s...' "$label"
  hook_bounded "$cmd"
  rc=$?
  elapsed=$((SECONDS - start))
  if [[ $rc -eq 0 ]]; then
    printf '\r  [✓] %s (%ss)\n' "$label" "$elapsed"
    return 0
  fi
  FAILED=1
  if [[ $rc -eq 124 || $rc -eq 137 ]]; then
    printf '\r  [✗] %s — TIMEOUT after %ss\n' "$label" "$GATE_TIMEOUT"
    echo "      A gate that cannot finish in ${GATE_TIMEOUT}s is a bug in the gate."
    echo "      Do NOT raise the ceiling — split, cache, or scope it (rules/timeouts.md)."
    echo "      Check first whether another build is competing for the CPU."
  else
    printf '\r  [✗] %s (exit %s, %ss)\n' "$label" "$rc" "$elapsed"
  fi
  printf '%s\n' "$HOOK_OUTPUT" | tail -30 | sed 's/^/      /'
  return "$rc"
}

# ── Cold-cache probe ────────────────────────────────────────────────────────
# A build directory with no compiled dependencies is cold. Cheap structural
# check: no mtime heuristics, no marker file to go stale, and it self-corrects
# the moment a build lands. `-maxdepth 1` and the early `head` keep it O(1)-ish
# on a warm tree, where this directory holds thousands of files.
#
# Mind WHICH directory. A crate that is a member of the ROOT workspace puts its
# artifacts in the root `target/`, not in `<crate>/target/` — that path exists
# and stays empty, and pointing at it makes this read cold forever.
hook_target_is_cold() {
  local dir="$1"
  [[ ! -d "$dir/debug/deps" ]] ||
    [[ $(find "$dir/debug/deps" -maxdepth 1 \( -name '*.rmeta' -o -name '*.rlib' \) 2>/dev/null | head -20 | wc -l) -lt 20 ]]
}

# ── Cold-cache warm-up ──────────────────────────────────────────────────────
# The ceiling governs steps that RUN things, where a hang is possible and killing
# is the right answer. Compilation is a different animal: it is bounded work that
# always terminates, and its cost is paid once per cache rather than once per
# invocation.
#
# Capping a genuinely COLD compile makes it unpassable rather than fast — the cap
# kills it before it finishes, so the next run starts cold again and no amount of
# retrying ever warms it. Measured on a cold worktree: one workspace's lint 53s
# (fits), the same workspace's build 8m11s (does not) — against a 300s cap that
# is clamped so it cannot be raised. And a linter can never share a compiler's
# cache, because the two write separate fingerprints, so no gate ordering avoids
# paying both. Every fresh worktree therefore hit a wall it could not clear.
#
# So warm once, uncapped, and let the capped gates run against a warm cache
# afterwards.
#
# Reach for this ONLY where the cold compile genuinely cannot fit the ceiling. A
# step measured to fit cold should just be bounded — a real ceiling that always
# applies beats an exemption that has to be reasoned about.
hook_warm_compile() {
  local label="$1" cmd="$2"
  printf "  [·] warming %s (cold cache, one-time)..." "$label"
  if bash -c "$cmd" >/dev/null 2>&1; then
    echo " ✓"
  else
    # A warm-up failure is not a verdict — the gate that follows re-runs the same
    # command WITH output captured and reports properly. Staying quiet here keeps
    # one failure from being reported twice in two different voices.
    echo " (see gate below)"
  fi
}
