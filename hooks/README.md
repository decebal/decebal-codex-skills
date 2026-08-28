# codex-guard lifecycle hooks

Three optional guardrails ship as one Rust binary: [`gates/rust/codex-guard`](../gates/rust/codex-guard).

| Subcommand | Event | Effect |
|---|---|---|
| `codex-guard infra-guard` | `PreToolUse`, `Bash` | Denies high-blast-radius infrastructure and destructive commands |
| `codex-guard bash-hygiene` | `PreToolUse`, `Bash` | Blocks compound shell commands/substitution; safely rewrites a redundant trailing `2>&1` |
| `codex-guard comment-hygiene` | `PostToolUse`, `apply_patch` | Returns added comment lines for immediate review |
| `codex-guard trust '<cmd>'` | manual | Records reviewed project-local wrapper content by SHA-256 |

## Install

```bash
cargo install --path gates/rust/codex-guard
cp configs/prod-guard-tokens.txt ~/.codex/prod-guard-tokens.txt
cp configs/hooks.json ~/.codex/hooks.json
```

Use an absolute binary path in `hooks.json` when `~/.cargo/bin` is absent from hook `PATH`.

Codex reviews non-managed hooks before running them. Treat hook configuration as executable code.

## Hook contract

Codex sends one JSON object on stdin. Relevant fields:

- `cwd`, `session_id`, `hook_event_name`, and `model`
- `tool_name`, `tool_use_id`, and `tool_input` for tool events
- `tool_input.command` for `Bash` and `apply_patch`

`codex-guard` emits nothing for allow. It uses supported Codex outputs:

- `permissionDecision: "deny"` for a blocked `PreToolUse`
- `permissionDecision: "allow"` plus `updatedInput` for a safe rewrite
- `additionalContext` for advisory `PostToolUse` feedback

Codex `PreToolUse` does not support `permissionDecision: "ask"`. Commands classified as confirmation-required are denied with a concise reason. Put native approval behavior in `.codex/rules/*.rules`.

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

## comment-hygiene

Codex sends patch text through `apply_patch`'s `tool_input.command`. Guard parses added lines per file, selects language comment markers by extension, excludes documentation comments, and returns bounded advisory context. It never pretends to undo completed edits.

## Limits

- Hooks are guardrails, not complete enforcement. Some specialized tool paths can opt out.
- Dynamic commands naming no recognizable tool can evade classification.
- Deep resolution stops outside Git root and at generated/dependency directories.
- `PostToolUse` cannot undo side effects.

## Tests

```bash
cargo test --manifest-path gates/rust/Cargo.toml -p codex-guard
```

Tests cover hook output shapes, Codex `apply_patch` payloads, decision matrix, wrapper resolution, rewrite safety, trust invalidation, and payload-size ceilings.

Current Codex schema: [official hooks documentation](https://learn.chatgpt.com/codex/hooks).
