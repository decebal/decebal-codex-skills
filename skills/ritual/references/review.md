# Review

A reviewer who did not build it reads the branch and the design, runs the repo's prove-it, and
runs what they can. Output: `docs/plans/YYYY-MM-DD-<slug>-code-review.md` on the branch, or a
review on the PR, named on the epic with its verdict:

```bash
cn task edit <epic> --append-description "ritual-review: <path or PR URL> clean" --toon
cn task edit <epic> --append-description "ritual-review: <path or PR URL> findings" --toon
```

The newest line wins, so a flow with findings stays in Review until a later clean review is named.

## Fresh context, always

The builder's account of the work is testimony. The reviewer starts from the design, the diff and
the repository, not from the build conversation:

- a `spawn_agent` reviewer in a fresh context, or a fresh Codex session (`codex exec`), given only
  the design path, the branch, the PR range (`<merge-base>..HEAD`) and the repo root;
- `cos` hires `reviewer` on the branch;
- a peer.

Never review in the builder's context.

## prove-it, every Review

The repo's own prove-it is the reviewer's verdict; `$pr-proof` stays Ship's author evidence. Both
run.

| | Codex | Claude Code |
|---|---|---|
| Detect, repo-local only | `test -f .agents/skills/prove-it/SKILL.md` | `test -f .claude/skills/prove-it/SKILL.md` |
| Census first, writes nothing | `$forge-prove-it --host codex --dry-run` | `/forge-prove-it --host claude --dry-run` |
| Forge | `$forge-prove-it --host codex --no-interview` | `/forge-prove-it --host claude --no-interview` |
| Refresh | `$forge-prove-it --host codex --update` | `/forge-prove-it --host claude --update` |

A user-level copy (`~/.codex/skills/prove-it`) does not count as the repo having one. On Codex the
forge writes `.agents/skills/prove-it/SKILL.md` (frontmatter `name` and `description` only) plus
`agents/openai.yaml`.

1. **Missing:** run the census, then the forge, in the flow's worktree. Never pass `--deep`.
2. **Open questions:** every `UNVERIFIED:` line the forge printed goes in the PR body under
   "Open questions". Nobody answers them in the flow; they print in every review until settled.
3. **Commit:** a forged prove-it is committed on the feature PR, in the author's one commit (forge
   writes files only; [ship.md](ship.md) collapses it in).
4. **Drift:** when the diff touches enforcement paths (hooks, CI workflows, gate scripts, lint
   or ratchet config, a path in the prove-it's `references/coverage-ledger.md`), run the refresh
   and commit its diff the same way.
5. **Run:** the fresh-context reviewer invokes `$prove-it` on the PR range and copies its verdict
   and `VERIFY BY HAND` lines into the code review. prove-it never posts `APPROVE`.

## What the reviewer checks

1. **Does it do what the design says?** Run it. A feature is proven by being exercised, not by
   its tests passing.
2. **Do the tests prove it?** Each test can fail; none passes by never running; the trophy
   layers in the plan file exist. Run each command as
   `allstack check --expect-exit <n> --seconds <2..270> -- <absolute path> <args>` and cite the
   receipt.
3. **Security:** `security-review` on the diff for anything that touches input, auth, secrets,
   crypto or the browser.
4. **Scope:** nothing outside the epic, no dead code, no comment narrating the fix.
5. **Drift:** the design still describes what was built. If not, update the design on the
   branch; the flow then reads `drift` and needs a new design review and approval before Ship.
6. **prove-it:** its findings are findings; its hand-checks go in the PR body under "Verify by
   hand".

Each finding: severity, file:line, the problem, the evidence (the command, its output or receipt),
the fix. Findings go back to Build as beads under the epic. Clean means Ship ([ship.md](ship.md)).
Optionally record the review: `allstack remember --kind review --source <code review>
--summary "<verdict>"`.
