# Codex plugins and skills

Codex plugins package one or more skills and may also bundle MCP servers, hooks, apps, and presentation assets. This repository is a skills-only plugin source through [`.codex-plugin/plugin.json`](../.codex-plugin/plugin.json).

## Inspect installed plugins

```bash
codex plugin list
codex plugin marketplace list
```

## Add from a configured marketplace

```bash
codex plugin marketplace add owner/repository --ref main
codex plugin add plugin-name@marketplace-name
```

A plugin repository is not automatically a marketplace. A marketplace source needs its own marketplace manifest and plugin entries.

## Install individual skills directly

For repository scope:

```bash
mkdir -p .agents/skills
cp -R /path/to/decebal-codex-skills/skills/typescript .agents/skills/
```

For user scope, copy or symlink to `~/.agents/skills/`. Codex discovers skill changes automatically; restart if a selector remains stale.

## Language tooling

Codex does not use provider-specific LSP plugin identifiers from other agents. Install language servers through normal system tooling and let repository commands or editor integrations use them. Do not invent `*-lsp@codex-plugins-official` names.

## Figma and other services

Use an installed plugin when available, or configure service MCP endpoints under `[mcp_servers.<name>]` in `config.toml`. Keep tokens in environment variables and use `codex mcp login` for OAuth-capable servers.

See [official skills documentation](https://developers.openai.com/codex/skills/) and [plugin documentation](https://learn.chatgpt.com/codex/build-plugins).
