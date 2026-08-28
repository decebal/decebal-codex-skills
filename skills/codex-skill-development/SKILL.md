---
name: codex-skill-development
description: Create or update skills in the decebal-codex-skills repository using official Codex skill authoring plus repository metadata, portability, licensing, validation, and installability gates. Use for work under this repository's skills directory; use official system skill-creator directly for unrelated personal skills.
---

# Codex Skill Development

Compose official `$skill-creator` with this repository's release gates. Do not
copy, replace, or shadow the official system skill.

## Author

1. Read `$skill-creator` completely and follow its creation or update workflow.
2. Choose a unique, lower-hyphen skill name. Check repository, user, system, and
   plugin skill names before adding it.
3. Keep `SKILL.md` concise. Move detailed reference material into files that
   `SKILL.md` links directly.
4. Add `agents/openai.yaml` with quoted strings, a 25-64 character
   `short_description`, and a one-sentence `default_prompt` that names the skill
   as `$skill-name`.
5. Bundle every required local runtime file. If an external runtime or tool is
   required, state installation and availability checks explicitly. Never ship
   instructions that reference absent paths.
6. Keep scripts deterministic and testable. Prefer Rust for repository tooling;
   use an existing official helper in its native language when it is the
   authoritative validator or generator.
7. Preserve portable licensing. Third-party-derived skill directories must
   carry required license and notice files, and root notices must identify them.
8. Update every caller, guide, default prompt, and relative link when renaming a
   skill.

## Validate

Run repository gate from repository root:

```bash
cargo run --manifest-path gates/rust/Cargo.toml -p skill-metadata-check -- \
  --skills-dir skills --plugin-manifest .codex-plugin/plugin.json
```

Then:

1. Run official `quick_validate.py` from installed system `$skill-creator`
   against each changed skill. This official Python helper is retained because
   it is Codex's authoritative compatibility check; repository automation stays
   Rust-first.
2. Run relevant script tests, repository tests, and `git diff --check`.
3. Resolve all local Markdown links and verify each referenced runtime file
   exists inside the installable skill package.
4. Search changed content for stale Claude-only names, paths, tools, settings,
   hooks, and invocation syntax.
5. Install from the exact candidate revision into an isolated destination or a
   collision-free user skill path, then validate installed bytes and metadata.

## Finish

Report skill name, trigger boundary, runtime dependencies, validation evidence,
install result, and retained or replaced colliding installations.
