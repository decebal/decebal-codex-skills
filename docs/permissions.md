# Permissions, sandboxing, and execution rules

Codex separates three controls:

1. `sandbox_mode` limits filesystem and network access.
2. `approval_policy` controls when Codex can ask before leaving those limits.
3. `.rules` files classify command prefixes requested outside the sandbox.

This differs from command-pattern allowlists. Do not translate an old `Bash(foo *)` entry into a broad `allow` rule without reviewing its external effect.

## Conservative project configuration

Create `.codex/config.toml` only in a trusted repository:

```toml
approval_policy = "on-request"
sandbox_mode = "workspace-write"

[sandbox_workspace_write]
network_access = false
exclude_slash_tmp = true
```

`workspace-write` permits normal work inside the repository while keeping other paths read-only. Enable network access only when project tasks require it.

## Approval policies

| Value | Behavior |
|---|---|
| `untrusted` | Runs only trusted commands without approval; prompts for others |
| `on-request` | Lets Codex request approval when needed |
| `never` | Never prompts; failures return to Codex |

`never` does not remove sandbox restrictions. `danger-full-access` removes filesystem/network sandboxing and needs separate risk review.

## Execution-policy rules

Place `.rules` files under `~/.codex/rules/` or `.codex/rules/` next to an active config layer. Rules apply when Codex requests to run a command outside the sandbox.

```python
prefix_rule(
    pattern = ["git", "push"],
    decision = "prompt",
    justification = "Pushing mutates a remote repository",
    match = ["git push", "git push origin feature/auth"],
    not_match = ["git status"],
)
```

Rules are prefix-based. Include `match` and `not_match` examples as inline tests. See [`../configs/default.rules`](../configs/default.rules).

## Security principles

- Keep sandbox writable roots narrow.
- Keep network disabled unless work needs it.
- Exclude secret-shaped environment variables from child processes.
- Prompt for remote writes, releases, cluster operations, privilege elevation, and destructive commands.
- Never rely on hooks as complete security enforcement; specialized tool paths can bypass normal hook coverage.
- Treat project-local `.codex/` configuration as executable policy. Review before marking repository trusted.
- Never store tokens in `config.toml`, `hooks.json`, or `.rules`; reference environment variables where supported.

## Files in this repository

- [`../configs/config.toml`](../configs/config.toml): baseline sandbox and approval settings
- [`../configs/default.rules`](../configs/default.rules): reviewed command-prefix examples
- [`../configs/hooks.json`](../configs/hooks.json): optional guard hooks

Current behavior: [official Codex configuration reference](https://developers.openai.com/codex/config-reference/), [sandbox documentation](https://learn.chatgpt.com/codex/sandboxing), and [rules documentation](https://learn.chatgpt.com/docs/agent-configuration/rules).
