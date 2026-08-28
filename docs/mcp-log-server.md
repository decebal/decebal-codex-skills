# MCP Log Server — runtime debugging

A local MCP server that watches a log directory and lets Codex read application
logs in real time. Instead of asking the user to copy-paste log output, Codex
queries the logs directly. This is a reusable pattern — not project-specific — and
the server itself is ~200 lines.

## Why

Some runtimes don't put logs where Codex can see them:

- **Tauri / desktop apps** — logs go to a file, not the terminal
- **Background services** — stdout isn't visible to the session
- **Debugging sessions** — Codex needs to see what actually happened at runtime,
  not a hand-transcribed excerpt

A log server closes that gap: the app writes to files, the server exposes them.

## Tools

| Tool | Purpose |
|------|---------|
| `list_logs` | List available log files with size and timestamp |
| `tail_log` | Get the last N lines of a log file |
| `get_errors` | Extract `ERROR` / `WARN` / `FATAL` lines |
| `search_logs` | Grep across log files |
| `log_stats` | Summarize error frequency and patterns |

## Configuration (`.mcp.json`)

```json
{
  "mcpServers": {
    "log-server": {
      "command": "bun",
      "args": ["path/to/mcp-log-server"],
      "env": {
        "LOG_DIR": "/tmp/app-logs"
      }
    }
  }
}
```

Point `LOG_DIR` at wherever the app writes. `command` can be any runtime that hosts
the server (`bun`, `node`, a compiled binary) — the transport is stdio.

## How it helps

A typical debugging loop:

1. `list_logs` — see what log files exist
2. `get_errors` — find recent errors
3. `tail_log` — read the last 50 lines of the active log
4. `search_logs` — grep for a specific pattern

### Example session

```
User: "The app crashes when I click Settings"

Codex: [list_logs]  → finds longhand-20260320.log
Codex: [get_errors] → finds "AllSource not initialized"
Codex: "Startup race condition — AllSource hasn't finished initializing when
         Settings tries to load config. Let me make the handler await init."
```

The crash is diagnosed from the actual runtime state, in one turn, with no
copy-paste round-trip.

## See also

General MCP server setup and the `.mcp.json` format: [mcp-servers.md](mcp-servers.md).
