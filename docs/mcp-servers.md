# MCP server integrations

Codex stores MCP server configuration in `config.toml`. Use `codex mcp` to add, inspect, authenticate, and remove servers instead of editing generated authentication state.

## HTTP server

```bash
codex mcp add example --url https://mcp.example.com/mcp
codex mcp login example
```

Equivalent configuration:

```toml
[mcp_servers.example]
url = "https://mcp.example.com/mcp"
bearer_token_env_var = "EXAMPLE_MCP_TOKEN"
startup_timeout_sec = 20
tool_timeout_sec = 45
enabled = true
```

## Stdio server

```bash
codex mcp add context7 -- npx -y @upstash/context7-mcp
```

Equivalent configuration:

```toml
[mcp_servers.context7]
command = "npx"
args = ["-y", "@upstash/context7-mcp"]
env_vars = ["LOCAL_TOKEN"]
```

## Tool policy

Limit exposed tools and require approval for sensitive operations:

```toml
[mcp_servers.example]
url = "https://mcp.example.com/mcp"
enabled_tools = ["read", "search", "write"]
default_tools_approval_mode = "prompt"

[mcp_servers.example.tools.read]
approval_mode = "approve"
```

## Project scope

Place repository-specific MCP settings in `.codex/config.toml` only for trusted projects. User-wide servers belong in `~/.codex/config.toml`. Plugin-provided servers are declared by plugin packaging and can be constrained under `plugins.<plugin>.mcp_servers.<server>`.

## Local log server

A useful desktop/background-app pattern is a stdio MCP server exposing bounded log operations such as `list_logs`, `tail_log`, `get_errors`, `search_logs`, and `log_stats`. See [`mcp-log-server.md`](mcp-log-server.md).

Current schema: [official Codex MCP documentation](https://developers.openai.com/codex/mcp/).
