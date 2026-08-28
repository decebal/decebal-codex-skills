# Decebal Codex Skills

[![gates](https://github.com/decebal/decebal-codex-skills/actions/workflows/gates.yml/badge.svg)](https://github.com/decebal/decebal-codex-skills/actions/workflows/gates.yml)
[![skill tests](https://github.com/decebal/decebal-codex-skills/actions/workflows/test-skills.yml/badge.svg)](https://github.com/decebal/decebal-codex-skills/actions/workflows/test-skills.yml)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**52 Codex skills. 21 Rust gate crates. 15 incident-backed rules. One kit for
making agent work repeatable, reviewable, and harder to fake.**

Prompts guide behavior. Compiled checks cover failures prose does not prevent:
skipped tests, broken layer boundaries, missing attribution, stale contracts,
unsafe hook commands, and gates that pass while scanning nothing.

This is the Codex-native sibling of
[`decebal-claude-skills`](https://github.com/decebal/decebal-claude-skills).
Provider-neutral engineering knowledge stays intact; Codex paths, metadata,
hooks, settings, instruction files, and command schemas are translated.

## Why this repo exists

Codex can follow a good instruction. Teams still need answers to harder
questions:

- Which workflow should trigger for this task?
- Which instruction belongs in `AGENTS.md`, a skill, or command policy?
- What catches the failure when an agent ignores prose?
- Does a green check prove the gate ran against anything?
- Can another engineer reproduce the workflow without this conversation?

Start small: select one skill, one rule, and one gate whose failure you can
demonstrate.

## What you get

| Layer | Contents | Use it for |
|---|---|---|
| Skills | 52 workflows under [`skills/`](skills/) | Planning, testing, browser work, architecture, Rust, TypeScript, deployment, media, and review |
| Chronis workflow | `$codex-prd` → `$codex-beads` → `cn ready/claim/done` | Turning requirements into dependency-aware execution with durable history |
| Agent instructions | Root [`AGENTS.md`](AGENTS.md) plus project [`templates/`](templates/) | Repository conventions Codex loads automatically |
| Portable rules | 15 focused [`rules/`](rules/) with incidents and exceptions | Selecting instruction fragments worth carrying into another codebase |
| Quality gates | 21-crate Rust workspace under [`gates/rust/`](gates/rust/) | Turning selected rules into fast, testable failures |
| Hooks and policy | [`hooks/README.md`](hooks/README.md), [`configs/hooks.json`](configs/hooks.json), and [`configs/default.rules`](configs/default.rules) | Reviewing lifecycle guardrails and execution-policy examples before adoption |
| Distribution | [`.codex-plugin/plugin.json`](.codex-plugin/plugin.json) | Packaging the collection as a skills-only Codex plugin |

## Quick start

Clone the repository and install only the skills you need:

```bash
git clone https://github.com/decebal/decebal-codex-skills.git
cd decebal-codex-skills

mkdir -p .agents/skills
cp -R skills/codex-prd .agents/skills/
cp -R skills/codex-beads .agents/skills/
```

Codex scans `.agents/skills` from the working directory to repository root.
For user-wide discovery, copy or symlink selected skills under
`~/.agents/skills/`. You can also ask built-in installer:

```text
$skill-installer Install codex-prd and codex-beads from
https://github.com/decebal/decebal-codex-skills
```

Then run one end-to-end workflow:

```bash
codex exec 'Use $codex-prd to write a PRD for OAuth login.'
codex exec 'Use $codex-beads to convert the OAuth PRD into Chronis beads.'
codex exec 'Work ready beads: claim, implement, verify, then mark done.'
```

Want the whole collection? Clone it, review
[`.codex-plugin/plugin.json`](.codex-plugin/plugin.json), then use built-in
`$plugin-creator` to add the directory to a local marketplace and test it before
sharing.

## Choose your layer

| Need | Put it here |
|---|---|
| Always-on repository guidance | `AGENTS.md` |
| Reusable workflow with scoped instructions or scripts | `skills/<name>/` |
| Human-readable engineering rule | `rules/<concern>.md` |
| Deterministic enforcement | `gates/` and CI or a git hook |
| Command approval policy | trusted `.codex/rules/*.rules` |
| Lifecycle behavior | trusted `hooks.json` or `[hooks]` config |

Markdown files under [`rules/`](rules/) are instruction fragments, not Codex
execution-policy `.rules` files. Codex does not import those fragments by name.
Use [`render-agent-docs`](gates/rust/render-agent-docs/) to embed a selected set
into `AGENTS.md`; do not load all 15 by reflex.

## What is enforced in this repository

Labels matter. “Included” does not mean “running,” and “running in CI” does not
mean “configured for your codebase.”

| Check | Status here | What that proves |
|---|---|---|
| Rust format, Clippy, and workspace tests | Enforced in this repository on pull requests and gate-related pushes | All 21 workspace members compile cleanly and their unit tests pass |
| Skill and plugin metadata | Enforced in the same workflow | Every skill frontmatter file, `agents/openai.yaml`, and plugin manifest passes repository validation |
| `codex-guard` | Real binary smoke-tested | Argument parsing and representative allow, rewrite, and deny outcomes work outside unit tests |
| Five config-driven gates | Run against a hostile fixture | Layer, forbidden-pattern, authority, attribution, and test-script gates catch known violations |
| Skill behavior | Two scripts tested in CI | `blog-image` and `web-video` pass executable tests; this is not a claim that all 52 skills have behavioral tests |
| Remaining gates | Reusable, opt-in | Binaries and tests ship here; adopters must configure paths and invoke them from CI or hooks |

[`gates/gates.toml`](gates/gates.toml) is a worked example for a Rust and
TypeScript monorepo. Its `apps/api`, `apps/web`, and `packages` paths do not
exist in this collection, so it is not active here. Likewise,
[`configs/hooks.json`](configs/hooks.json) and
[`configs/default.rules`](configs/default.rules) are review-before-copy
examples. Codex runs non-managed hooks only after trust review, and execution
policy only loads from an active trusted config layer.

Both CI workflows run on every pull request. Push triggers are path-filtered:
README-only pushes do not rerun them. Badges therefore show latest applicable
workflow run, not proof about every documentation commit.

### Which rules fit here?

| Recommendation | Rules | Reason |
|---|---|---|
| Use for this repository | `git-discipline`, `evidence-discipline`, `comments`, `definition-of-done`, `dependency-hygiene`, `testing-gates`, `token-efficiency` | Matches a shared skills collection with Rust tooling, dependencies, tests, and release evidence |
| Add when work demands it | `debugging-discipline`, `agent-parallelism`, `timeouts` | Useful during diagnosis or parallel work; extra always-on context otherwise |
| Ship for consuming applications, not active here | `layer-boundaries`, `error-channels`, `event-streams`, `ui-remote-states`, `data-over-binary` | These require application layers, user-facing remote state, event UX, or tenant data planes this repository does not have |

Three gates could fit this repository after deliberate configuration:
`check-test-hangs` for Rust test trees, `vendor-attribution-check` adapted to
third-party skill bundles, and `render-agent-docs` if root `AGENTS.md` becomes a
generated artifact. None should be enabled until its scan paths match real files
and CI proves it catches a planted violation.

## A workflow that leaves evidence

```text
$codex-prd
    ↓ requirements and acceptance criteria
$codex-beads
    ↓ epic, tasks, dependencies, verification checklist
cn ready → cn claim → implement → verify → cn done
    ↓
git history + CI result + Chronis history
```

See [`skills-guide/codex-workflow.md`](skills-guide/codex-workflow.md) and
[`skills-guide/chronis-git-best-practices.md`](skills-guide/chronis-git-best-practices.md).

## Repository map

```text
.
├── skills/          52 reusable Codex skills
├── gates/           Rust gates, hook glue, fixtures, and adoption guide
├── rules/           portable instruction fragments
├── templates/       AGENTS.md starters by project shape
├── skills-guide/    skill authoring and Chronis workflows
├── configs/         Codex configuration, hooks, and command-policy examples
├── hooks/           codex-guard setup and trust guidance
└── docs/            permissions, MCP, hooks, and operating notes
```

## Verify the repository

```bash
cargo fmt --manifest-path gates/rust/Cargo.toml --all --check
cargo clippy --manifest-path gates/rust/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path gates/rust/Cargo.toml --workspace
cargo run --manifest-path gates/rust/Cargo.toml -p skill-metadata-check -- \
  --skills-dir skills --plugin-manifest .codex-plugin/plugin.json
bash tests/run.sh
```

See [`gates/README.md`](gates/README.md) for gate contracts and adoption notes,
and [`rules/README.md`](rules/README.md) for rule selection guidance.

## Codex references

- [Build skills](https://developers.openai.com/codex/build-skills)
- [Build plugins](https://developers.openai.com/codex/build-plugins)
- [Layer instructions with AGENTS.md](https://developers.openai.com/codex/agent-configuration/agents-md)
- [Command execution rules](https://developers.openai.com/codex/agent-configuration/rules)
- [Lifecycle hooks](https://developers.openai.com/codex/hooks)

## License and provenance

Original work is MIT licensed. Imported material keeps its upstream license;
see [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md). Port history starts at
[`decebal/decebal-claude-skills`](https://github.com/decebal/decebal-claude-skills).
