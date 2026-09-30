# codex-guard lifecycle hooks

Six optional guardrails ship as one Rust binary: [`gates/rust/codex-guard`](../gates/rust/codex-guard).

| Subcommand | Event | Effect |
|---|---|---|
| `codex-guard infra-guard` | `PreToolUse`, `Bash` | Denies high-blast-radius infrastructure and destructive commands |
| `codex-guard bash-hygiene` | `PreToolUse`, `Bash` | Blocks compound shell commands/substitution; safely rewrites a redundant trailing `2>&1` |
| `codex-guard pr-guard` | `PreToolUse`, `Bash` | Denies a second PR, or a new branch, while a PR this session created is still open in that repo |
| `codex-guard prompt-number` | `PreToolUse`, `apply_patch` | Denies creating `.prompts/NNN-*.md` when `.prompts/.numbers/NNN` does not exist |
| `codex-guard rule-scope` | `PreToolUse`, `apply_patch` and `Bash`; `PostCompact` | Adds a path-scoped rule to model context the first time a session touches a file it covers; forgets what it sent when the history is compacted |
| `codex-guard comment-hygiene` | `PostToolUse`, `apply_patch` | Returns added comment lines for immediate review |
| `codex-guard trust '<cmd>'` | manual | Records reviewed project-local wrapper content by SHA-256 |

## Install

```bash
cargo install --path gates/rust/codex-guard
cp configs/prod-guard-tokens.txt ~/.codex/prod-guard-tokens.txt
cp configs/hooks.json ~/.codex/hooks.json
mkdir -p ~/.codex/agent-rules
```

Use an absolute binary path in `hooks.json` when `~/.cargo/bin` is absent from hook `PATH`.

Codex reviews non-managed hooks before running them. Treat hook configuration as executable code.

## Hook contract

Codex sends one JSON object on stdin. Relevant fields:

- `cwd`, `session_id`, `transcript_path`, `hook_event_name`, and `model`
- `tool_name`, `tool_use_id`, and `tool_input` for tool events
- `tool_input.command` for `Bash` and `apply_patch`

`codex-guard` emits nothing for allow. It uses supported Codex outputs:

- `permissionDecision: "deny"` for a blocked `PreToolUse`
- `permissionDecision: "allow"` plus `updatedInput` for a safe rewrite
- `additionalContext` with no `permissionDecision` for `PreToolUse` context (`rule-scope`); the call runs
- `additionalContext` for advisory `PostToolUse` feedback

Codex `PreToolUse` does not support `permissionDecision: "ask"`. Commands classified as confirmation-required are denied with a concise reason. Put native approval behavior in `.codex/rules/*.rules`.

State files live under `CODEX_HOME` (default `~/.codex`), named by the payload's `session_id` reduced to `[A-Za-z0-9_-]`.

## infra-guard

Literal classification catches live-service mutation, IAM changes, Terraform apply/destroy/state surgery, storage deletion, namespace/PVC deletion, package publishing, protected-branch force-pushes, dangerous recursive deletion, privilege elevation, and broad process termination.

Deep scanning follows project-local wrapper chains:

- `[cd <dir> &&] bash|sh|source|. <path>`
- direct `./script.sh` and relative script paths
- `make [-C <dir>] <target>`
- `bun|npm|pnpm run <script>` with working-directory flags

`INFRA_GUARD_DEPTH` selects wrapper depth (default `5`; `0` disables deep scanning while retaining literal checks). Read-only infrastructure commands stay silent.

Production tokens load from nearest Git root's `.codex/prod-guard-tokens.txt`, then session working directory, then `~/.codex/prod-guard-tokens.txt`. A command touching production and non-production together is denied.

For a reviewed wrapper chain:

```bash
codex-guard trust 'make deploy-staging'
```

Trust covers resolved script contents, Make recipe text, and package-script text. Any change invalidates hash.

Set `INFRA_GUARD_OFF=1` in hook environment only for an explicit temporary bypass. It disables this guard, so remove it immediately after reviewed operation.

## bash-hygiene

One Bash tool call should contain one command. Guard blocks `&&`, `||`, `;`, unprotected newlines, command/process substitution, backticks, `tee`, combined redirects, and optional temp paths outside `CODEX_SCRATCH_DIR`.

Trailing `2>&1` with no other redirect is removed through `updatedInput`; Codex already captures both streams. Piped or file-redirected forms remain blocked because removing merge changes semantics.

## pr-guard

One session keeps at most one open PR per GitHub repo. While a PR this session created is open, the guard denies, in that repo:

- `gh pr create` and `gh pr new`, with the repo taken from `-R`, `--repo`, `GH_REPO` (in `OWNER/REPO`, `github.com/OWNER/REPO` or URL form), or the `origin` remote of `cwd`
- branch creation: `git switch -c|--create`, `git checkout -b`, `git worktree add -b`, and `git branch <name>`, including `git -C <dir>` forms
- the resetting forms `git switch -C`, `git checkout -B`, `git worktree add -B` and `git branch -f`, only when the branch does not exist yet; moving an existing branch, such as `git branch -f main origin/main`, passes

Every segment of a chained command is checked, after `VAR=value`, `env`, `command`, `rtk` and `rtk proxy` prefixes come off. Heredoc bodies are data and are not read as commands. `--help`, quoted mentions, and the listing, deleting and renaming forms of `git branch` (combined short flags such as `-dr` included) pass.

The deny reason names the repo, the open PR's number, title and head branch. The change belongs on that branch; a second PR needs the user's own words, and the user runs it by prefixing the command with `!` in the Codex prompt.

**The session ledger.** Every `gh pr create` the guard allows appends `{"repo","head","at"}` to `$CODEX_HOME/pr-guard/<session_id>.jsonl`. `head` is the `--head`/`-H` value, else a branch created earlier in the same command, else the current branch of `cwd`; it is resolved before any `gh` call. A create is recorded before it runs, so when `head` already has an open PR at that moment, its number is stored as `preexisting` and never counted as the session's. `--dry-run` and `--web` are not recorded. The session's open PRs in a repo are the open `gh pr list --author @me` PRs there whose head branch the ledger holds for that repo. Creating a new ledger file deletes ledger files older than 14 days in the same directory, looking at no more than 500 entries.

**The transcript is a fallback.** In code mode, the output of a long command can arrive on a later tool call than the one that started it, so joining a creation to its output by `call_id` misses PRs. The ledger reads no output. The rollout at `transcript_path` is scanned only when the session has no ledger, which means it began before the hook existed. A PR counts when a command in the call parses as `gh pr create` (from a `function_call`'s `cmd`/`command`, or a code-mode call's `cmd:` string literals) and the same call's output carries its `github.com/<owner>/<repo>/pull/<n>` URL; an output saying the PR already exists is skipped. The scan stops at 64 MiB or the deadline.

**Fails open** when the repo cannot be resolved, `gh` is missing, unauthenticated or offline, the ledger or transcript is unreadable, or the lookups pass the deadline. One 8-second deadline covers every `git` and `gh` child; a child still running at the deadline is killed, then reaped. A create with no resolvable repo or head, or no `session_id`, is allowed and not recorded.

Limits:

- A PR created outside this hook's view (the web UI, another tool, a denied-then-hand-run command) is not in the ledger.
- A PR someone else opens later on a head branch this session recorded counts as the session's.
- The head is read from `cwd` when `--head` is absent, which is also what `gh` does.

## prompt-number

`.prompts/` is gitignored, so each worktree sees its own near-empty copy, and a number picked by looking at the directory can collide with one another checkout already issued. `prompt-id alloc <slug>` claims the number under `.prompts/.numbers/` first.

For every `*** Add File:` and `*** Move to:` path in the patch whose file name is `NNN-*.md` directly inside a `.prompts` directory, resolved against `cwd`, the guard denies unless `.prompts/.numbers/NNN` exists. The deny reason names the number and the fix: run `prompt-id alloc <slug>` (from [decebal-claude-skills](https://github.com/decebal/decebal-claude-skills), `gates/rust/prompt-id`) and write to the path it prints.

Allowed without a claim:

- `*** Update File:` on any path
- a target that already exists
- a store with no `.numbers/` directory, which never adopted the allocator
- a path below `.prompts/` rather than directly in it, such as `completed/NNN-x.md`
- a `Move to` from a file in the same store with the same number, which renames without issuing one

## rule-scope

Codex has no path-scoped instructions: AGENTS.md loads whole. `rule-scope` delivers a rule whose YAML frontmatter lists `paths:` globs the first time a session touches a matching file, as `additionalContext` on that call. It never denies, spawns no process, and exits silently on any error.

```markdown
---
paths:
  - "**/*_tests.rs"
  - "**/tests/**"
---

# Testing

...
```

**Sources**, each a directory of `*.md`, in this order:

1. `$CODEX_HOME/agent-rules/` for global rules. Never `$CODEX_HOME/rules/`: that is Codex's execution-policy directory.
2. `<root>/.agents/rules/`
3. `<root>/.claude/rules/`

`<root>` is the nearest ancestor of `cwd` holding `.git`. A rule file without `paths:` is ignored here; [render-agent-docs](../gates/rust/render-agent-docs) inlines it into AGENTS.md and lists the path-scoped ones by name when they come from one of these directories. A rule reached twice (a symlinked `.agents` → `.claude`, or the same file installed globally and in the repo) is delivered once.

**Candidates.** For `apply_patch`, the `Add File`, `Update File` and `Move to` paths. For `Bash`, up to 64 non-flag words that contain `/` or read as a file name with an extension (`--flag=value` keeps its value; URLs are dropped).

**Matching.** A glob matches the candidate relative to `<root>`; a path outside it is matched as written. `**`, `*`, `?` and `{a,b}` are supported, and every glob is anchored at the root. A glob with no `/` does not match at any depth: `Cargo.lock` is the root file, `**/Cargo.lock` is every one.

**Once per session, until compaction.** Delivered rule paths are appended to `$CODEX_HOME/rule-scope/<session_id>`, and a listed rule is not sent again in that session. Registered on `PostCompact` (see [`configs/hooks.json`](../configs/hooks.json)), the same subcommand deletes that file, because a compaction summarises the delivered text away; each rule is then sent again the next time a matching file is touched. A rule is recorded when the hook prints it, whether or not another hook then denies the call. Creating a new state file deletes state files older than 14 days (no more than 500 entries examined).

**Cap.** The context text starts with one line saying the rules govern every file their paths match for the rest of the session, then `### <file stem> (<path>)` and the body, frontmatter stripped, for each rule. The whole text stays within 16,000 bytes; a rule that does not fit is listed as `read <path>` instead.

## comment-hygiene

Codex sends patch text through `apply_patch`'s `tool_input.command`. Guard parses added lines per file, selects language comment markers by extension, excludes documentation comments, and returns bounded advisory context. It never pretends to undo completed edits.

## Limits

- Hooks are guardrails, not complete enforcement. Some specialized tool paths can opt out.
- Dynamic commands naming no recognizable tool can evade classification.
- Deep resolution stops outside Git root and at generated/dependency directories.
- `PostToolUse` cannot undo side effects.
- `rule-scope` sees only paths a patch header or a shell word names; a file reached through a glob, a variable or a script is not a candidate.

## Tests

```bash
cargo test --manifest-path gates/rust/Cargo.toml -p codex-guard
```

Tests cover hook output shapes, Codex `apply_patch` payloads, decision matrix, wrapper resolution, rewrite safety, trust invalidation, the pr-guard ledger and rollout fixtures (including output that arrives on a later call), prompt claims across added and moved files, rule frontmatter and glob translation, once-per-session delivery, the context cap, and payload-size ceilings.

Current Codex schema: [official hooks documentation](https://learn.chatgpt.com/codex/hooks).
