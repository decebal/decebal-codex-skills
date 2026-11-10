# Git discipline

Portable. No stack assumptions beyond git + a forge with squash or rebase merges.
The incidents behind these rules: `~/.codex/rules-reference/git-discipline-incidents.md`.

## Before you commit: confirm the branch is LIVE (first thing, every session)

**The most expensive git failure: committing onto a merged (dead) branch.** 61
commits once landed on a branch after its PR had merged, and none reached the
trunk.

- **At the start of every session, and after any resume, before the first
  commit:**
  ```bash
  B=$(git branch --show-current)
  gh pr list --head "$B" --state all --json number,state,title
  ```
  - **`MERGED`** → the branch is DEAD. Do not commit. `git fetch origin main`,
    then (with permission) `git switch -c <type>/<name> origin/main`.
  - **`OPEN`** → live, carry on.
  - **No PR** and not the trunk → work waiting to strand. Open a PR now.
- **Re-check on long sessions** and whenever a push behaves oddly: a branch can
  merge out from under you.
- **Never inherit a branch blindly.** A session that starts on a branch it did
  not create runs the check first.
- **Automate it:** `gates/sh/check-branch-not-merged.sh` is the pre-push gate. It
  runs before any bypass path and fails open only when `gh` is unavailable.

## One open PR per session

**A session keeps at most one open PR per repo. Everything the session does
while it is open lands on it**: a fix the PR needs to go green (a red check it
inherits from the trunk included), a follow-up on the same surface, discovered
work, and the next thing the user asks for. A second open PR needs the user's
own words, and the user runs it themselves (`! gh pr create`). Never offer a
split ("two PRs?") as an option. Once the PR merges, the next request starts
the next PR.

**The cost, 2026-09-29:** one session created 29 PRs; one component took four
in a day, the last a 4-line fix for the red check on the PR still open beside
it. The rule was written down and loaded, and did not hold.

- **Enforced by `codex-guard pr-guard`** (PreToolUse, Bash): denies
  `gh pr create` / `gh pr new` and branch creation (`switch -c`, `checkout -b`,
  `worktree add -b`, `branch <name>`) while a PR this session created is still
  open in that repo. It records each PR it allows in a per-session ledger under
  `$CODEX_HOME/pr-guard/`, checks them with one `gh pr list`, and allows on any
  error or at an 8 s deadline.

## One PR = one commit, merged by rebase

**A PR reaches the trunk as exactly ONE commit, and the forge rebase-merges.**
The commit that lands is then the commit that was reviewed and tested; a squash
merge invents a new commit nobody ran anything against, which is what makes
ancestry checks lie.

- **Keep the branch at one commit.** Amend as you go, or collapse before review:
  ```bash
  git reset --soft "$(git merge-base origin/<trunk> HEAD)"
  git commit -F <message-file>
  ```
  Reset to the **merge-base**, never a commit count: `HEAD~11` swallows whatever
  the trunk merged in.
- **Prove the collapse lost nothing:** `git diff <old-head> HEAD` must be empty.
- **Push with `--force-with-lease`**, never bare `--force`.
- **Set the forge to rebase-merge only**, so the merge button enforces this.
- **Re-run the gates afterwards.** The collapsed commit is a new SHA with no
  status.

### The carve-out, stated exactly

| Rewriting | Allowed? |
|---|---|
| Your own feature branch, unmerged, no one else's commits on it | **Yes**: amend and collapse freely, `--force-with-lease` |
| A branch carrying commits by another person or session | **No.** Ask them: their reflog is the only copy |
| The trunk | **No** |
| A branch whose PR already merged | **No**: it is dead; branch fresh |

A second author name here means stop:
```bash
git log --format='%an' "$(git merge-base origin/<trunk> HEAD)"..HEAD | sort -u
```

## Squash and rebase merges break ancestry checks

Both land the work under a new SHA, so anything that reasons about ancestry lies.

- **Did a branch merge?** `gh pr list --head <branch> --state all` → `MERGED`.
  Never `git log main..branch` or `git branch --merged`.
- **A merged branch is dead.** Don't continue on it, rebase it, or cherry-pick
  from it.
- **Is my work on the trunk?** Read the code: `git show origin/main:<path>`.
  Cross-check with a second signal before concluding anything is missing
  ([evidence-discipline.md](evidence-discipline.md)); a wrong "it's missing"
  triggers a needless re-land.

## Branch discipline

- **NEVER push to the trunk.** No exceptions.
- **NEVER create a branch without explicit permission** in the current message.
- **NEVER bypass the hooks**: no `--no-verify`, no skip env var.
- **Before EVERY push, read both:**
  ```bash
  git branch --show-current                # must not be the trunk
  git rev-parse --abbrev-ref @{upstream}   # must not be origin/<trunk>
  ```
  A worktree branch made from `origin/main` tracks it by default: push with
  `git push -u origin HEAD:<type>/<name>`.
- **Local name = remote name.** Never `git push origin local:different-remote`.
- **Never rewrite history that is not yours** (see the carve-out).
- **Never `git stash`.** Use a branch.
- **Never run destructive git on uncommitted work**: no `git checkout <ref> -- <path>`,
  `git restore`, or `git reset --hard`. Read other refs with `git show` / `git diff`.

## Multi-agent coordination

- **Isolate the tree, not the branch.** One `git worktree` per agent removes
  file-stomping and build-lock contention ([agent-parallelism.md](agent-parallelism.md)).
  Own branch: `git worktree add -b <type>/<name> <path> origin/main`. Shared
  branch: `git worktree add --detach <path> origin/<branch>`, commit, then
  `git push origin HEAD:<branch>`.
- **Stage by explicit path**, never `git add -A` / `git add .`, and check
  `git diff --cached --stat` before committing: the tree may hold another
  agent's files.
- **Expect push races** (`cannot lock ref`): fetch, fast-forward onto the new
  tip, push again, and confirm by re-reading the remote ref, never by the exit
  code.
- **Claim a task before starting it** in the tracker.
- **Only the orchestrator collapses, once, at the end.** While siblings still
  push to a shared branch, nobody amends or force-pushes it.

## Commits

- `<type>(<scope>): <description>`, lowercase, imperative, under 72 chars; type
  one of `feat` `fix` `refactor` `test` `docs` `chore`. The body explains why.
- **Never a task-tracker id (`t-…`, `bd-…`, `PROJ-123`) in source**, comments
  included. Commit messages and `docs/` are exempt. `gates/rust/check-no-id-refs`
  enforces it.
- **One PR, one commit.** Discovered work, blocking fixes included, goes on the
  open PR. The collapsed commit's message lists what the PR carries.
- **The build must pass at the commit that merges.**
- **Don't commit others' work, and don't commit unless asked.**
- **Commit in the foreground, check `git log --oneline -1` shows YOUR commit,
  then push.** A commit chained to a push can fail its hook and ship the branch
  at its parent: an empty branch that cannot open a PR.

## Pull requests

- **Title** in conventional commit format, under 70 chars. **Body:** summary
  bullets, test-plan checklist, link to the design doc.
- **Open the PR with the first pushed commit.** A branch with commits and no PR
  is invisible; at ~5 commits with no PR, stop and open one.
- **Not ready while the branch has more than one commit.** Collapse, re-run the
  gates on the new SHA, then mark it ready.
- **One feature = one PR**, unless it depends on another open PR's code; then it
  is a stack layer (below). Branching from a convenient local branch is not a
  dependency: that PR targets the trunk.
- **Size is never a reason to split or pause** ([definition-of-done.md](definition-of-done.md)).
- **A branch reused after its PR merged needs a NEW branch**, not a new PR on it.

## Stacked PRs

A PR may be based on another open PR's branch when it needs that PR's code.

- **Open a layer as a draft** with `--base <parent branch>`. It is marked ready
  only once its base is the trunk.
- **One commit per author per layer.** A second author's work is its own layer
  on top, not a commit on the first author's branch.
- **Land bottom-up.** Merge the bottom layer, restack the next onto the trunk
  (`git rebase --onto origin/<trunk> <old-parent-head> <branch>`), gate it, then
  merge it. Rebase and squash merges put the parent on the trunk under new SHAs,
  so an unrestacked layer replays the parent's commits.
- **Review each layer's own diff**, against its parent, and gate each layer's
  head. A gate verdict on an upper layer goes stale whenever the parent moves.
- **Enforced by `stack-guard`** in `codex-guard` and `claude-guard` (PreToolUse,
  Bash). With `git config stack-guard.gate-context <status>` set, a merge also
  needs that status at `success` on the exact head, merged with
  `--match-head-commit`.

The cost behind it, 2026-11-10: with stacking banned, one contributor had five
hand-built chains open in one repo, none gated, several layers carrying more than
one commit by one author, and one carrying a stale copy of its parent's commit.
