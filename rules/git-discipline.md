# Git discipline

Portable. No stack assumptions beyond git + a forge with squash merges.

## Before you commit: confirm the branch is LIVE (do this first, every session)

**The most expensive git failure there is: committing onto a squash-merged (dead)
branch.** A multi-session, 61-commit effort was once piled onto a branch *after*
its PR had already squash-merged — none of it reached the trunk, and it was only
caught much later. Prevent it:

- **At the start of every session, and again after any resume, run the liveness
  check before the first commit:**

  ```bash
  B=$(git branch --show-current)
  gh pr list --head "$B" --state all --json number,state,title
  ```

  - PR shows **`MERGED`** → the branch is **DEAD**. STOP. Do not commit. Fetch
    and branch fresh: `git fetch origin main` then (with permission)
    `git switch -c <type>/<name> origin/main`. Commits added to a branch whose PR
    is merged strand off-trunk.
  - PR **`OPEN`** → live, carry on.
  - **No PR at all** and the branch isn't the trunk → untracked work waiting to
    strand. Open a PR now, or confirm the plan before piling on more commits.

- **Re-check on long sessions.** A branch can be squash-merged out from under you
  mid-session. If you've been working a while, or a push behaves oddly, re-run
  the check before pushing again.

- **Never inherit a branch blindly.** A session that starts already on some
  `feat/*` branch you didn't create must run the liveness check before trusting
  it.

- **Automate it.** `gates/sh/check-branch-not-merged.sh` is the drop-in
  pre-push gate. It runs *before* any skip/bypass path and hard-fails a push to a
  dead branch. It fails **open** when `gh` is unavailable (CI/headless), so local
  dev — where the mistake actually happens — is always guarded.

## Squash merges break every ancestry check

A squash merge replaces a branch's commits with one new-SHA commit. Everything
that reasons about ancestry then lies:

- **To check if a branch merged:** `gh pr list --head <branch> --state all` and
  look for `MERGED`. **Not** `git log main..branch`, **not** `git branch --merged`
  — both wrongly report the commits as "not on main".
- **A merged branch is a dead branch.** Don't continue on it, don't rebase it,
  don't cherry-pick from it. Never rebase a squash-merged branch onto the trunk:
  its commits are either dropped as already-upstream or produce false conflicts.
- **To decide "is my work on the trunk?", read the CODE, not the ancestry.**
  `git show origin/main:<path>`, or check the trunk out in a worktree and open
  the file. Cross-check with a second signal before concluding anything is
  missing — see [evidence-discipline.md](evidence-discipline.md). A wrong "it's
  missing" triggers a needless, risky re-land: cherry-picks, duplicate PRs, exactly
  the churn the dead-branch bug already caused once.

## Branch discipline

- **NEVER push to the trunk.** All work goes through feature branches and PRs. No
  exceptions, no "just this once", no CI-only changes.
- **NEVER create a branch without explicit permission** in the current message.
  Work on the current branch or ask.
- **NEVER bypass the hooks** — no `--no-verify`, no skip env var. If the pre-push
  hook blocks you, that is the system working.
- **Before EVERY push, verify two things and READ the output:**
  ```bash
  git branch --show-current            # must not be the trunk
  git rev-parse --abbrev-ref @{upstream}   # must not be origin/<trunk>
  ```
  A worktree branch created from `origin/main` **tracks origin/main by default**.
  Push it explicitly: `git push -u origin HEAD:feat/<name>`.
- **One branch = one concern.** If concurrent work sits in the tree, stage only
  your files by name.
- **Local name = remote name.** `git push -u origin $(git branch --show-current)`.
  Never `git push origin local:different-remote`.
- **Never rewrite shared history** — no `--force`, no `--amend` on pushed commits,
  no interactive rebase on a published branch.
- **Never `git stash`.** Use a branch.
- **Never run destructive git on uncommitted work** — no `git checkout <ref> -- <path>`,
  no `git restore`, no `git reset --hard`. Use `git show` / `git diff` to READ
  state from another ref.

## Multi-agent coordination

Several agent sessions often run against one repo at once. The 61-commit pile-up
was fundamentally a coordination failure — several sessions committing to the
same inherited, already-merged branch.

- **Isolate the TREE, not the branch.** Sharing a *branch* was never the bug;
  sharing a *working tree* was. Give each agent its own `git worktree`: one move
  that removes source-tree file-stomping AND build-lock contention, because each
  worktree gets its own build directory. See
  [agent-parallelism.md](agent-parallelism.md).
  - *Own branch:* `git worktree add -b <type>/<name> <path> origin/main`.
  - *Shared branch* (a branch can be checked out in only ONE worktree, so detach):
    `git worktree add --detach <path> origin/<branch>`, commit, then
    `git push origin HEAD:<branch>`. On a non-fast-forward rejection, fetch +
    rebase your commit and retry.
- **Stage by explicit path, never `git add -A` / `git add .`** — the tree may hold
  another agent's uncommitted files. Between `git add` and `git commit`, run
  `git diff --cached --stat` and confirm the index is exactly yours.
- **Expect push races.** Parallel pushes to one branch get compare-and-swap
  rejections (`cannot lock ref … is at X but expected Y`). Fetch, fast-forward
  your commits onto the new tip, push again. Confirm success by RE-READING the
  remote ref (`git rev-parse origin/<branch>`), never by the command's exit code —
  a piped push reports the **pipe's** status, not git's.
- **Claim a task before starting it** in whatever tracker you use; the claim is
  the signal to other agents that the work is taken.

## Conventional commits

`<type>(<scope>): <description>` — lowercase, imperative, under 72 chars.

| Type | When |
|------|------|
| `feat` | New feature or capability |
| `fix` | Bug fix |
| `refactor` | Neither fixes nor adds |
| `test` | Adding or updating tests |
| `docs` | Documentation only |
| `chore` | Build, CI, deps, tooling |

Body explains **why**, not what.

**Never put a task-tracker id (`t-…`, `bd-…`, `PROJ-123`) in SOURCE code**, comments
and doc-comments included. Ids are local, they churn, and they mean nothing to a
reader without the tracker. Cite a decision record instead, or just describe the
change. Commit messages and `docs/` are exempt. `gates/sh/check-no-id-refs.sh`
enforces this on source only.

## Clean commit history

- **One logical change per commit.** Don't bundle unrelated fixes.
- **The build must pass at every commit.**
- **Don't commit others' work** — `git add` your files by name.
- **Don't commit unless explicitly asked.** Leave changes for review.
- **Commit in the FOREGROUND, verify it landed, THEN push.** Never background or
  `&&`-chain a commit together with a push. A commit that fails its pre-commit
  hook exits non-zero, but a chained push runs anyway and ships the branch at its
  PARENT commit — an *empty* branch with none of your work, which then cannot open
  a PR (`No commits between main and <branch>`). Run `git commit`, confirm
  `git log --oneline -1` shows YOUR commit and not the parent SHA, and only then
  push. Backgrounding the *push alone* is fine when the gate is slow.

## Pull requests

- **Title** — conventional commit format, under 70 chars.
- **Body** — Summary (bullets), Test plan (checklist), link to the design doc.
- **Open the PR early, not at the end.** A branch with commits and no PR is
  invisible: nothing shows it isn't merging, and work silently accumulates
  off-trunk — half of how the 61-commit pile-up happened. Once a feature branch
  has its first pushed commit, open the PR. If a branch reaches ~5 commits with no
  PR, stop and open one.
- **One feature = one PR.** Never stack a PR on an open PR's branch.
- **Size is never a reason to split or pause.** See
  [definition-of-done.md](definition-of-done.md).
- **A branch reused after its PR merged needs a NEW branch, not a new PR** — a
  fresh PR from a squash-merged branch produces a mega-diff of dead commits plus
  new ones.
