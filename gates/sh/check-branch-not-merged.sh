#!/bin/bash
# Dead-branch guard: refuse to push to a branch whose PR was already
# squash-merged.
#
# A squash merge replaces a branch's commits with a single new-SHA commit. The
# branch is then DEAD — any further commit pushed to it never reaches the trunk.
# A whole 61-commit, multi-session effort was lost exactly this way, by continuing
# to commit to a branch after its PR had merged. This is the durable backstop so
# it cannot recur (rules/git-discipline.md).
#
# Usage:
#   check-branch-not-merged.sh [branch]     # defaults to the current branch
#   TRUNK=master check-branch-not-merged.sh # if your trunk is not `main`
#
# Exit 0 = branch is live, or the check could not run. Exit 1 = the branch's PR
# is MERGED; do not commit or push here.
#
# Call it from pre-push BEFORE any skip/bypass path — it is the one gate that
# must not be skippable, and it needs no toolchain, which is why it is shell.
#
# FAIL-OPEN by design: if `gh` is missing or unauthenticated, or the API errors
# (CI, headless, offline), the check SKIPS rather than blocking a legitimate push
# on tooling absence. Only a definitive "PR is MERGED" blocks.
set -uo pipefail

TRUNK="${TRUNK:-main}"
BRANCH="${1:-$(git branch --show-current 2>/dev/null || true)}"

# Detached HEAD, or the trunk itself → not this gate's job. The trunk is
# protected separately, and a detached push carries no branch identity to check.
if [[ -z "$BRANCH" || "$BRANCH" == "$TRUNK" ]]; then
  exit 0
fi

if ! command -v gh >/dev/null 2>&1; then
  echo "check-branch-not-merged: gh not found — skipping (fail-open)"
  exit 0
fi

# Any MERGED PR whose head is this branch means the branch is dead. jq guards the
# empty-array case so no match yields an empty string, not "null".
MERGED=$(gh pr list --head "$BRANCH" --state merged --json number,title \
  --jq 'if length > 0 then "\(.[0].number)\t\(.[0].title)" else "" end' 2>/dev/null) || {
  echo "check-branch-not-merged: gh query failed — skipping (fail-open)"
  exit 0
}

if [[ -n "$MERGED" ]]; then
  NUM="${MERGED%%$'\t'*}"
  TITLE="${MERGED#*$'\t'}"
  echo ""
  echo "  ✗ BLOCKED: branch '$BRANCH' was already squash-merged (PR #$NUM)."
  echo "    \"$TITLE\""
  echo ""
  echo "    A squash-merged branch is DEAD — commits pushed here never reach $TRUNK."
  echo "    Start fresh instead:"
  echo "      git fetch origin $TRUNK && git switch -c <type>/<name> origin/$TRUNK"
  echo ""
  echo "    (Backstop for the 61-commit dead-branch pile-up. Do not bypass.)"
  echo ""
  exit 1
fi

exit 0
