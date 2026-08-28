# Hooks configuration

Codex hooks run commands or MCP tools during lifecycle events. Hooks may add context, inspect tool calls, deny supported operations, or rewrite supported tool input.

## Discovery

Codex loads `hooks.json` or inline `[hooks]` tables next to active config layers. Common locations:

- `~/.codex/hooks.json`
- `~/.codex/config.toml`
- `.codex/hooks.json` in a trusted project
- plugin lifecycle configuration

Multiple matching command hooks launch concurrently. One hook cannot prevent another matching hook from starting.

## Core events

| Event | Typical use |
|---|---|
| `SessionStart` | Add bounded project context |
| `UserPromptSubmit` | Inspect or enrich prompt context |
| `PreToolUse` | Deny or rewrite supported tool calls |
| `PermissionRequest` | Allow or deny an approval Codex already needs |
| `PostToolUse` | Review results and add feedback |
| `Stop` | Run a final validation reminder |
| `SessionEnd` | Short cleanup or telemetry |

See [`../configs/hooks.json`](../configs/hooks.json) for compiled guard wiring.

## Command hook

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "^Bash$",
        "hooks": [
          {
            "type": "command",
            "command": "codex-guard infra-guard",
            "timeout": 10,
            "statusMessage": "Checking infrastructure command"
          }
        ]
      }
    ]
  }
}
```

Commands receive one JSON object on stdin and run with session `cwd`. Resolve repository-local binaries from Git root; Codex can start from nested directories.

## Supported output

For `PreToolUse`, deny:

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "deny",
    "permissionDecisionReason": "Blocked by repository policy."
  }
}
```

Rewrite:

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow",
    "updatedInput": { "command": "git status" }
  }
}
```

For `PostToolUse`, return bounded `additionalContext`. Blocking feedback cannot undo completed side effects.

`permissionDecision: "ask"` is not supported for Codex `PreToolUse`; use execution-policy `.rules` to request approvals.

## Best practices

- Keep synchronous hooks fast and silent on clean paths.
- Cap model-visible output; hook context compounds with other instructions.
- Avoid secrets in hook output because oversized output can spill to disk.
- Use `PreToolUse` for narrow policy, not broad instruction repetition.
- Test real stdin/stdout payloads and hook configuration.
- Treat hooks as defense in depth, not a complete security boundary.

Current contract: [official Codex hooks documentation](https://learn.chatgpt.com/codex/hooks).
