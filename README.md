# Codex Best Practices & Tools

Codex-native sibling of [`decebal-claude-skills`](https://github.com/decebal/decebal-claude-skills): reusable skills, AGENTS.md templates, lifecycle-hook examples, configuration, and Rust quality gates.

## What changed for Codex

This repository preserves provider-neutral engineering material and translates runtime-specific surfaces:

- `CLAUDE.md` becomes layered `AGENTS.md` guidance.
- Skills install under `.agents/skills` and include Codex UI metadata.
- Multi-skill distribution uses `.codex-plugin/plugin.json`.
- Settings examples use `config.toml`, `hooks.json`, and `.codex/rules/*.rules`.
- Hook commands and tests target Codex payload/output schemas.
- `claude-guard`, `claude-prd`, and `claude-beads` become `codex-guard`, `codex-prd`, and `codex-beads`.
- Non-interactive examples use `codex exec`.
- Markdown rule fragments are rendered into `AGENTS.md`; Codex execution-policy `.rules` remain separate.

## Structure

```text
.
├── .codex-plugin/plugin.json   # skills-only Codex plugin manifest
├── AGENTS.md                   # repository instructions loaded by Codex
├── skills/                     # 52 reusable skills
├── rules/                      # portable instruction fragments
├── templates/                  # AGENTS.md project templates
├── skills-guide/               # authoring and Chronis workflows
├── configs/                    # config.toml, hooks.json, and execution-policy examples
├── hooks/                      # codex-guard installation and hook guidance
├── gates/                      # Rust quality gates and git-hook glue
└── docs/                       # Codex configuration and integration guides
```

## Install

### All skills as plugin source

Clone repository, then use it as a local skills-only plugin while developing or package it through a Codex marketplace.

```bash
git clone https://github.com/decebal/decebal-codex-skills.git
cd decebal-codex-skills
```

### Selected skills for one repository

Codex scans `.agents/skills` from working directory to repository root:

```bash
mkdir -p .agents/skills
cp -R /path/to/decebal-codex-skills/skills/codex-prd .agents/skills/
cp -R /path/to/decebal-codex-skills/skills/codex-beads .agents/skills/
```

For user-wide discovery, copy or symlink selected skill directories under `~/.agents/skills/`.

Invoke explicitly with `$skill-name`, or let Codex match skill descriptions.

## Chronis workflow

```text
$codex-prd  →  $codex-beads  →  cn ready/claim/done  →  codex exec
```

Example:

```bash
codex exec 'Create a PRD for user authentication with OAuth.'
codex exec 'Convert tasks/prd-user-auth.md to Chronis beads.'
codex exec 'Work ready beads: use cn ready --toon, claim each, implement, verify, then cn done.'
```

See [`skills-guide/codex-workflow.md`](skills-guide/codex-workflow.md).

## Guard hooks

Build and install compiled guard:

```bash
cargo install --path gates/rust/codex-guard
cp configs/prod-guard-tokens.txt ~/.codex/prod-guard-tokens.txt
cp configs/hooks.json ~/.codex/hooks.json
```

Review and trust hook configuration before enabling it. See [`hooks/README.md`](hooks/README.md).

## Configuration

- [`configs/config.toml`](configs/config.toml): conservative Codex defaults
- [`configs/hooks.json`](configs/hooks.json): lifecycle-hook wiring
- [`configs/default.rules`](configs/default.rules): execution-policy examples
- [`docs/permissions.md`](docs/permissions.md): sandbox, approvals, and rules
- [`docs/mcp-servers.md`](docs/mcp-servers.md): MCP configuration

## Verification

```bash
cargo fmt --manifest-path gates/rust/Cargo.toml --all --check
cargo clippy --manifest-path gates/rust/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path gates/rust/Cargo.toml --workspace
cargo run --manifest-path gates/rust/Cargo.toml -p skill-metadata-check -- \
  --skills-dir skills --plugin-manifest .codex-plugin/plugin.json
```

Official Codex plugin and skill validators were also run during porting. CI uses
repository-local Rust validation across every skill and plugin metadata.

## Principles

1. AGENTS.md carries project instructions; closer files override broader files.
2. Skills hold reusable workflows; descriptions determine implicit triggering.
3. Chronis is dependency-aware execution checklist and durable task history.
4. Sandbox and approval policy are separate controls.
5. Lifecycle hooks provide guardrails, not a complete security boundary.
6. A rule without a gate remains advice.
7. Preserve evidence: verify resulting artifact, not only command exit status.

## Attribution

Ported from [`decebal/decebal-claude-skills`](https://github.com/decebal/decebal-claude-skills). See [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) for imported skill licensing and modifications.
