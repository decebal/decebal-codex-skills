#!/bin/bash
# agent-worktree.sh — one isolated git worktree per agent.
#
# Several agents sharing ONE checkout causes two distinct pains: source-tree
# file-stomping (one agent's uncommitted edits clobbering another's) AND build
# lock contention (every concurrent build serializes on one lock; a hung test
# thrashes it for everyone). A worktree gives each agent its OWN source tree AND
# its OWN build directory — separate locks — which removes both at once.
# See rules/agent-parallelism.md.
#
# Commands:
#   create <name>              own-branch: new branch `<name>` off origin/$TRUNK
#   create <name> --on <br>    shared-branch: a DETACHED worktree on origin/<br>
#                              (a branch can be checked out in only ONE worktree,
#                              so shared use means a detached HEAD; commit, then
#                              `git push origin HEAD:<br>`)
#   seed <name|path>           re-run seeding on an existing worktree (idempotent)
#   list                       show all worktrees
#   remove <name> [--force]    remove the worktree for <name>
#
# `create` also SEEDS the worktree from `gates/worktree-seed.conf`. Seeding is
# VERIFIED on disk step by step: anything that did not happen is named, and an
# incomplete worktree makes the command exit NON-ZERO so it can never be mistaken
# for a ready one. The predecessor seeded one directory while printing a cheerful
# summary, and three worktrees were created with no dependencies at all — each
# agent found out when a gate failed.
#
# NEVER set a shared build-output directory (CARGO_TARGET_DIR and friends). That
# only MOVES the single lock to a common place; it does not remove it, and it
# reintroduces exactly the thrash separate worktrees eliminate.
#
# Worktrees are created under $AGENT_WORKTREE_ROOT (default: a sibling
# `<repo>-agents/` directory) so they never appear in the repo's own status.
#
# WHY this one is shell and the logic gates are Rust: it is git, `cp` and package
# installs — process orchestration, where the shell IS the natural glue, and this
# implementation is the one whose failure modes were actually paid for.
set -euo pipefail

TRUNK="${TRUNK:-main}"

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" || {
  echo "agent-worktree: not inside a git repo" >&2
  exit 1
}

# Every seed SOURCE lives in the PRIMARY checkout. `--show-toplevel` is whatever
# tree we were invoked from — run this from inside a worktree and that tree may
# be missing the very artifacts we are about to copy. `git worktree list` always
# reports the main working tree first, so resolve the source from there.
MAIN_CHECKOUT="$(git worktree list --porcelain | awk '/^worktree /{print substr($0, 10); exit}')"
[[ -n "$MAIN_CHECKOUT" && -d "$MAIN_CHECKOUT" ]] || MAIN_CHECKOUT="$REPO_ROOT"

WT_ROOT="${AGENT_WORKTREE_ROOT:-$(dirname "$REPO_ROOT")/$(basename "$REPO_ROOT")-agents}"
SEED_CONF="${AGENT_WORKTREE_SEED_CONF:-$REPO_ROOT/gates/worktree-seed.conf}"
GUARD="$REPO_ROOT/gates/sh/check-branch-not-merged.sh"

# A branch name may contain '/', fine for a branch but not for one path segment.
dir_for() { printf '%s' "$1" | tr '/' '-'; }

usage() {
  # Print the header comment block — everything between the shebang and the first
  # non-comment line. (A hard-coded line range silently truncates the moment the
  # header grows.)
  awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "$0"
  exit "${1:-0}"
}

cmd_create() {
  local name="${1:-}"; shift || true
  [[ -z "$name" ]] && { echo "agent-worktree create: <name> required" >&2; exit 1; }
  local on_branch=""
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --on) on_branch="${2:-}"; shift 2 || { echo "--on needs a branch" >&2; exit 1; } ;;
      *) echo "agent-worktree create: unknown arg '$1'" >&2; exit 1 ;;
    esac
  done

  # Declared and assigned separately (SC2155): `local x="$(cmd)"` returns
  # `local`'s status, never the command's — the exact masking that let the
  # predecessor report seeding it had not done.
  local path
  path="$WT_ROOT/$(dir_for "$name")"
  [[ -e "$path" ]] && { echo "agent-worktree: '$path' already exists" >&2; exit 1; }
  mkdir -p "$WT_ROOT"

  if [[ -n "$on_branch" ]]; then
    # Shared-branch mode is for FEATURE branches. Never let it target the
    # protected trunk — the push hint would read `git push origin HEAD:$TRUNK`,
    # exactly the forbidden move (rules/git-discipline.md).
    case "$on_branch" in
      "$TRUNK"|main|master)
        echo "agent-worktree: refusing --on '$on_branch' — shared-branch mode is for feature branches, never the protected trunk." >&2
        exit 1 ;;
    esac
    if [[ -x "$GUARD" ]]; then
      "$GUARD" "$on_branch" || { echo "agent-worktree: '$on_branch' is a dead branch — aborting." >&2; exit 1; }
    fi
    # Resolve a start point. A branch that has never been pushed has no
    # `origin/<branch>`, so requiring one rejects every branch before its first
    # push — the common case, since a shared branch is usually created locally and
    # handed to agents before anything lands on the remote.
    local start_ref=""
    if git fetch origin "$on_branch" 2>/dev/null; then
      start_ref="origin/$on_branch"
    elif git show-ref --verify --quiet "refs/heads/$on_branch"; then
      # Detached at the LOCAL tip. Still detached, not checked out: a branch lives
      # in one worktree at a time, and this one is likely claimed already.
      start_ref="$on_branch"
    else
      echo "agent-worktree: no branch '$on_branch' — not on origin, not local." >&2
      echo "  Create it first (git branch $on_branch origin/$TRUNK), or check the spelling." >&2
      exit 1
    fi
    git worktree add --detach "$path" "$start_ref"
    echo "✓ worktree (shared branch '$on_branch', detached at $start_ref) → $path"
    if [[ "$start_ref" != origin/* ]]; then
      echo "  note: '$on_branch' is not on origin yet — started from the local branch."
    fi
    echo "  cd $path"
    echo "  # edit, commit, then push to the shared branch:"
    echo "  git push origin HEAD:$on_branch"
    echo "  # if rejected non-fast-forward: git fetch && git rebase origin/$on_branch, retry"
  else
    # Own-branch mode: fresh branch off the latest trunk.
    #
    # --no-track is load-bearing, not tidiness. `git worktree add -b <name> <path>
    # origin/$TRUNK` inherits tracking from the start point, so the new branch's
    # upstream becomes origin/$TRUNK and a bare `git push` in that worktree targets
    # the protected trunk — the one move rules/git-discipline.md forbids outright.
    # With no upstream a bare push fails closed ("no upstream configured"), and the
    # -u in the hint below sets the correct one on first push.
    git fetch origin "$TRUNK"
    git worktree add -b "$name" --no-track "$path" "origin/$TRUNK"
    echo "✓ worktree (new branch '$name' off $TRUNK) → $path"
    echo "  cd $path"
    echo "  # work, then open the PR early:"
    echo "  git push -u origin $name && gh pr create --base $TRUNK"
  fi
  echo "  (isolated build directory — do NOT set a shared CARGO_TARGET_DIR)"
  echo "  remove when done: $0 remove $name"

  # Seeding is the slow part, so it runs AFTER the hints are on screen. It is also
  # the part that used to lie, so its status is the command's status.
  if ! seed_worktree "$path"; then
    exit 1
  fi
}

# ---------------------------------------------------------------------------
# Seeding
# ---------------------------------------------------------------------------
# A fresh worktree is missing EVERYTHING gitignored, and every gap surfaces only
# at gate time — after the work is done. The steps live in worktree-seed.conf so
# this script stays repo-agnostic:
#
#   install <dir|.> <label> <verify-rel-path>
#   clone   <rel-path> <required|optional> [must-contain]
#   build   <dir> <verify-rel-path> <command…>
#
# Every step is VERIFIED ON DISK after it runs, the summary names anything that
# did not happen, and an incomplete worktree exits NON-ZERO. A script that lies
# about seeding is worse than one that does nothing — silence gets checked, a
# green tick does not.

SEED_DEST=""
SEED_OK=()
SEED_BAD=()
SEED_SKIP=()

seed_ok()   { SEED_OK+=("$1");   printf '  ✓ %s\n' "$1"; }
seed_skip() { SEED_SKIP+=("$1"); printf '  – %s\n' "$1"; }
seed_note() { printf '  ! %s\n' "$1"; }
seed_bad() { # seed_bad <what-did-not-happen> <how-to-fix-it>
  SEED_BAD+=("$1"$'\n      fix: '"$2")
  printf '  ✗ %s\n' "$1" >&2
}

# rules/timeouts.md: no step may run longer than 5 minutes. A seeding step that
# hangs must be KILLED and REPORTED — a silent hang is the same lie as a silent
# skip.
SEED_TIMEOUT_BIN=""
for _t in timeout gtimeout; do
  if command -v "$_t" >/dev/null 2>&1; then SEED_TIMEOUT_BIN="$_t"; break; fi
done
guarded() { # guarded <seconds> <cmd…>
  local secs="$1"; shift
  if [[ -n "$SEED_TIMEOUT_BIN" ]]; then
    "$SEED_TIMEOUT_BIN" -k 15 "$secs" "$@"
  else
    "$@"
  fi
}

# Stops at the first entry instead of listing a 100k-file build directory.
dir_nonempty() { [[ -n "$(find "$1" -mindepth 1 -maxdepth 1 -print -quit 2>/dev/null)" ]]; }

# Clone one gitignored artifact from the main checkout.
#
# `cp -c` is an APFS clonefile (copy-on-write), so 159 MB "copies" in about a
# second and costs no additional disk until a file diverges. That is what makes
# seeding cheap enough to do unconditionally. On a non-APFS filesystem the flag
# is rejected and the plain `cp -R` fallback runs.
#
# The copy lands on a staging path and is MOVED into place only once it has
# succeeded, so a half-finished copy can never leave a directory that merely
# LOOKS seeded.
seed_copy() { # seed_copy <rel-path> <required|optional> [must-contain]
  local rel="$1" mode="$2" must="${3:-}"
  local src="$MAIN_CHECKOUT/$rel" dest="$SEED_DEST/$rel"
  local fix="cp -Rc '$src' '$dest'"

  if [[ -e "$dest" ]]; then
    seed_ok "$rel (already present)"
    return 0
  fi
  if [[ ! -d "$src" ]]; then
    if [[ "$mode" == required ]]; then
      seed_bad "$rel — NOT SEEDED: nothing to clone, $src does not exist" \
        "produce it in the main checkout first (it is gitignored build output), then: $0 seed <name>"
    else
      seed_skip "$rel — nothing to clone ($src does not exist)"
    fi
    return 0
  fi

  local stage="$dest.seeding.$$"
  rm -rf "$stage"
  mkdir -p "$(dirname "$dest")"

  # `local out` is declared SEPARATELY on purpose: `local out="$(cmd)"` returns
  # `local`'s status, not the command's, which is exactly how a failing step gets
  # reported as a success.
  local out
  if ! out="$(guarded 300 cp -Rc "$src" "$stage" 2>&1)"; then
    rm -rf "$stage"
    if ! out="$(guarded 300 cp -R "$src" "$stage" 2>&1)"; then
      rm -rf "$stage"
      seed_bad "$rel — NOT SEEDED: copy failed (${out:-cp exited non-zero})" "$fix"
      return 0
    fi
  fi
  if ! dir_nonempty "$stage"; then
    rm -rf "$stage"
    seed_bad "$rel — NOT SEEDED: the copy produced an empty tree" "$fix"
    return 0
  fi
  if ! mv "$stage" "$dest" 2>/dev/null; then
    rm -rf "$stage"
    seed_bad "$rel — NOT SEEDED: could not move the staged copy into place" "$fix"
    return 0
  fi
  if [[ -n "$must" && ! -e "$dest/$must" ]]; then
    seed_bad "$rel — SEEDED BUT INCOMPLETE: $must is missing" "$fix"
    return 0
  fi
  # Keep the desktop search indexer out of build output. It indexes every
  # artifact it can see, and a seeded build directory arrives with tens of
  # thousands of them — measured once at 33,210 files in one deps directory, with
  # the indexer at 91% CPU for a week while worktrees came and went. The marker
  # has to be re-dropped per worktree, because each gets its own directory.
  case "$rel" in
    target|*/target) : > "$dest/.metadata_never_index" 2>/dev/null || true ;;
  esac
  seed_ok "$rel"
}

# A package install for one directory.
#
# --frozen-lockfile first: an install that rewrites the lockfile leaks that diff
# into the agent's commit. If the lockfile genuinely cannot satisfy the manifest
# we retry unfrozen and SAY so, rather than leave the tree with no dependencies.
seed_install() { # seed_install <rel-dir|.> <label> <verify-rel-path>
  local rel="$1" label="$2" must="$3"
  local dir="$SEED_DEST"
  [[ "$rel" == "." ]] || dir="$SEED_DEST/$rel"
  local fix="(cd '$dir' && bun install)"

  if [[ ! -f "$dir/package.json" ]]; then
    seed_bad "$label install — NOT RUN: no package.json at $dir" "$fix"
    return 0
  fi

  local out
  if ! out="$(cd "$dir" && guarded 300 bun install --frozen-lockfile 2>&1)"; then
    if ! out="$(cd "$dir" && guarded 300 bun install 2>&1)"; then
      printf '%s\n' "$out" | tail -20 >&2
      seed_bad "$label install — FAILED (output above)" "$fix"
      return 0
    fi
    seed_note "$label: lockfile was stale, installed unfrozen — check the lockfile before you commit"
  fi
  if [[ ! -e "$dir/$must" ]]; then
    seed_bad "$label install — ran but $must is still missing" "$fix"
    return 0
  fi
  seed_ok "$label install ($must)"
}

# Build an artifact that could not be cloned. Skipped when it is already there,
# so a `clone … optional` line above it wins when the main checkout has one.
seed_build() { # seed_build <rel-dir> <verify-rel-path> <command…>
  local rel="$1" must="$2"; shift 2
  local dir="$SEED_DEST"
  [[ "$rel" == "." ]] || dir="$SEED_DEST/$rel"
  local fix="(cd '$dir' && $*)"

  if [[ -e "$dir/$must" ]]; then
    seed_ok "$rel/$must (already present)"
    return 0
  fi
  local out
  if ! out="$(cd "$dir" && guarded 300 bash -c "$*" 2>&1)"; then
    printf '%s\n' "$out" | tail -20 >&2
    seed_bad "$rel/$must — BUILD FAILED (output above)" "$fix"
    return 0
  fi
  if [[ ! -e "$dir/$must" ]]; then
    seed_bad "$rel/$must — build ran but the artifact is missing" "$fix"
    return 0
  fi
  seed_ok "$rel/$must (built)"
}

seed_worktree() { # seed_worktree <worktree-path>
  SEED_DEST="$1"
  SEED_OK=(); SEED_BAD=(); SEED_SKIP=()

  echo ""
  echo "seeding $SEED_DEST"
  echo "  (sources: $MAIN_CHECKOUT)"

  if [[ ! -f "$SEED_CONF" ]]; then
    echo "  – no $SEED_CONF — nothing declared to seed."
    echo "    A worktree with no dependencies fails its gates for reasons that have"
    echo "    nothing to do with the diff. Declare the steps; see gates/README.md."
    return 0
  fi
  echo "  (steps:   $SEED_CONF)"

  local kind a b c rest
  while read -r kind a b c rest; do
    case "$kind" in
      ''|'#'*) continue ;;
      install) seed_install "$a" "$b" "$c" ;;
      clone)   seed_copy "$a" "$b" "$c" ;;
      build)   seed_build "$a" "$b" "$c $rest" ;;
      *) seed_bad "unknown seed step '$kind' in $SEED_CONF" "use install | clone | build" ;;
    esac
  done < "$SEED_CONF"

  echo ""
  if [[ ${#SEED_BAD[@]} -eq 0 ]]; then
    local tail_note=""
    [[ ${#SEED_SKIP[@]} -gt 0 ]] && tail_note=", ${#SEED_SKIP[@]} with nothing to do"
    echo "✓ worktree seeded — ${#SEED_OK[@]} step(s) verified on disk$tail_note"
    return 0
  fi

  local total=$((${#SEED_OK[@]} + ${#SEED_BAD[@]}))
  echo "✗ WORKTREE IS NOT READY — ${#SEED_BAD[@]} of $total seeding step(s) did not complete:" >&2
  local f
  for f in "${SEED_BAD[@]}"; do
    printf '    • %s\n' "$f" >&2
  done
  echo "  Gates run in this worktree will fail for reasons that have nothing to do" >&2
  echo "  with your diff. Fix the steps above, then re-run:" >&2
  echo "    $0 seed $SEED_DEST" >&2
  return 1
}

cmd_list() { git worktree list; }

# Re-seed an existing worktree. Idempotent — every step no-ops when its artifact
# is already there — so it is also the repair path for a worktree whose seeding
# partly failed.
cmd_seed() {
  local name="${1:-}"
  [[ -z "$name" ]] && { echo "agent-worktree seed: <name-or-path> required" >&2; exit 1; }
  local path="$name"
  [[ -d "$path" ]] || path="$WT_ROOT/$(dir_for "$name")"
  [[ -d "$path" ]] || { echo "agent-worktree: no worktree at '$path'" >&2; exit 1; }
  if ! seed_worktree "$path"; then
    exit 1
  fi
}

cmd_remove() {
  local name="${1:-}"; shift || true
  [[ -z "$name" ]] && { echo "agent-worktree remove: <name> required" >&2; exit 1; }
  local force=""
  [[ "${1:-}" == "--force" ]] && force="--force"
  local path
  path="$WT_ROOT/$(dir_for "$name")"
  [[ -d "$path" ]] || { echo "agent-worktree: no worktree at '$path'" >&2; exit 1; }

  # Never remove someone else's in-flight work. A process check tells you whether
  # A build is running, not whose; uncommitted changes or an unpushed commit mean
  # somebody is mid-flight (rules/agent-parallelism.md).
  if [[ -z "$force" ]]; then
    if [[ -n "$(git -C "$path" status --porcelain 2>/dev/null)" ]]; then
      echo "agent-worktree: '$path' has uncommitted changes — refusing." >&2
      echo "  Push them first. --force discards them." >&2
      exit 1
    fi
  fi

  df -h / | tail -1 | awk '{print "  disk before: " $4 " free"}'
  rm -rf "$path/target"
  git worktree remove $force "$path"
  git worktree prune
  # `du` reports APPARENT size; a clonefile-seeded build directory shares most of
  # its blocks with the tree it came from, so deleting it frees only what
  # diverged. Measured once: 50 G by `du`, 2 GiB actually returned. `df` is the
  # only honest number.
  df -h / | tail -1 | awk '{print "  disk after:  " $4 " free"}'
  echo "✓ removed worktree $path"
}

case "${1:-help}" in
  create) shift; cmd_create "$@" ;;
  seed)   shift; cmd_seed "$@" ;;
  list)   shift; cmd_list "$@" ;;
  remove) shift; cmd_remove "$@" ;;
  help|-h|--help) usage 0 ;;
  *) echo "agent-worktree: unknown command '${1:-}'" >&2; usage 1 ;;
esac
