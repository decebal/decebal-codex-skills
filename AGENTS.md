# Codex Best Practices Repository

Documentation, reusable skills, configuration examples, and Rust quality gates for Codex.

## Repository rules

- Treat `skills/` as plugin source. Every skill requires `SKILL.md` with only `name` and `description` in YAML frontmatter.
- Treat `.codex-plugin/plugin.json` as distribution metadata. Validate it after changing plugin structure.
- Use `AGENTS.md` for automatically loaded project instructions. Codex does not expand Markdown `@` imports; render selected rule fragments into generated instruction files.
- Keep lifecycle-hook examples compatible with Codex hook schemas and canonical tool names.
- Prefer Rust for repository tooling and durable scripts. Existing third-party or media utilities may retain their original runtime when rewriting them adds no value.
- Preserve attribution and license notices when porting upstream material.
- Keep docs concise and link to official OpenAI documentation for changing Codex behavior.

## Required checks

```bash
cargo fmt --manifest-path gates/rust/Cargo.toml --all --check
cargo clippy --manifest-path gates/rust/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path gates/rust/Cargo.toml --workspace
```

Also run plugin and skill validation described in `README.md`.
