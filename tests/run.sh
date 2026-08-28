#!/usr/bin/env bash
# Runs every skill test. Each test self-skips (exit 0) when its deps are missing,
# so this is safe to run anywhere; non-zero exit means a real failure.
set -euo pipefail
cd "$(dirname "$0")"

rc=0
for t in test_*.sh; do
  echo "── $t ──"
  if bash "$t"; then :; else rc=1; fi
done

echo
[ "$rc" -eq 0 ] && echo "ALL SKILL TESTS PASSED (or skipped)" || echo "SOME SKILL TESTS FAILED"
exit "$rc"
