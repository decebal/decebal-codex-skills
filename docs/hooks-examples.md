# Hook examples

Copy selected entries into `hooks.json` next to an active Codex config layer.

## PreToolUse command policy

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "^Bash$",
        "hooks": [
          {
            "type": "command",
            "command": "codex-guard bash-hygiene",
            "timeout": 5,
            "statusMessage": "Checking shell command"
          }
        ]
      }
    ]
  }
}
```

## SessionStart context

Command must emit valid hook JSON when it has context to add. Plain text stdout is ignored for this event.

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "^startup$",
        "hooks": [
          {
            "type": "command",
            "command": "project-context-hook",
            "timeout": 5,
            "additionalContextLimit": 1000
          }
        ]
      }
    ]
  }
}
```

Example success output:

```json
{
  "hookSpecificOutput": {
    "hookEventName": "SessionStart",
    "additionalContext": "Release branch is frozen; do not change public API."
  }
}
```

## Stop validation feedback

```json
{
  "hooks": {
    "Stop": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "verify-stop-hook",
            "timeout": 30,
            "statusMessage": "Checking completion gates"
          }
        ]
      }
    ]
  }
}
```

Return `continue: false` with `stopReason` only when deterministic validation proves work incomplete. Avoid self-referential prompts that repeatedly block stopping.

See [`hooks.md`](hooks.md) and [official Codex hooks documentation](https://learn.chatgpt.com/codex/hooks).
