# Git discipline: the incidents behind the rules

The rules live in [`rules/git-discipline.md`](../../rules/git-discipline.md).
This file holds the cases they came from.

## The 61-commit dead branch

A multi-session effort of 61 commits was piled onto a branch *after* its PR had
already squash-merged. None of it reached the trunk, and it was caught much
later. It was a coordination failure as much as a git one: several sessions
committed to the same inherited, already-merged branch, and the branch carried
work for a long time with no open PR, so nothing showed it was not merging.
That is why the liveness check runs before the first commit of every session,
why an inherited branch is never trusted blindly, and why a PR is opened with
the first pushed commit.

## 2026-09-21: collapsing 11 commits

A PR sat at 11 commits, two of them trunk merges. Collapsing it by hand meant
resetting to the merge-base: `HEAD~11` would have swallowed both merges, and a
branch that merged the trunk twice has more parents than subjects. The standing
"never rewrite shared history" line made the routine request read as forbidden,
and it took a round-trip to establish that a single-author unmerged branch was
never what that ban was for. Both halves became the carve-out table.

## 2026-09-29: one session, 29 PRs

One repo merged 244 PRs in 14 days, 47 on its busiest day. One agent session's
transcript recorded 29 PRs it had created. A single UI component took four PRs
in one day, each fixing what the previous one left:

1. a crash on a repeated tag,
2. removing the tags and category chips,
3. making the picker use the settings page's list and search,
4. a 4-line test-label fix in another app.

The fourth was opened while the third was open and red because of exactly those
two labels, so the third could not go green without it. For the first two, the
agent had offered the user "both, two PRs" as the recommended option. The rule
line it followed read *"an unrelated fix is a different PR, not a second
commit"*, against a loaded rule saying discovered work is in scope. The same
miss had been recorded twice before in the agent's own notes, which is why the
fix is a hook (`codex-guard pr-guard`) and not another sentence.

## The backgrounded commit that shipped an empty branch

A backgrounded `commit; ...; push` chain hid a formatter failure in the
pre-commit hook. The commit exited non-zero, the chained push ran anyway, and
it shipped the branch at its PARENT commit: an empty branch with none of the
work, which then could not open a PR (`No commits between main and <branch>`).
