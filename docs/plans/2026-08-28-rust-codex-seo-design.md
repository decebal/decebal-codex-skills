# Rust codex-seo design

Date: 2026-08-28

## Goal

Upgrade existing `codex-seo` skill from routing-only compatibility layer to an evidence-led Codex workflow backed by deterministic Rust analysis. Rename documented `Search & Growth` inventory category to **SEO** and keep exactly two skills there:

- `codex-seo`
- `app-store-optimization`

Skill paths stay flat under `skills/` for Codex discovery and plugin packaging.

## Provenance and scope

Workflow is a Rust-first adaptation of `AgriciDaniel/claude-seo` revision `a1480c7e590b16001bd9dc1627eacdcd44d580f9` (v2.2.5, MIT). Upstream contains 25 skills and 53 Python scripts. Migration consolidates shared decisions into one Codex skill and ports bounded deterministic page, sitemap, and drift analysis to Rust.

Credentialed Google, Bing, DataForSEO, Moz, maps, backlink, analytics, performance, rendering, and image-generation capabilities remain routed evidence inputs. Missing integration output is `setup required`; Codex never fabricates it. Existing installed SEO specialists remain preferred for their narrow domains and can compose with Rust output.

`app-store-optimization` already ships its Rust validator and remains unchanged except category/docs/CI composition updates.

## Runtime

`skills/codex-seo/scripts/` is an independent locked Cargo workspace producing `codex-seo`.

Commands:

- `audit`: inspect local HTML or fetch one public HTTP(S) page; emit JSON or Markdown.
- `sitemap`: inspect local or public XML sitemap shape, URLs, duplicates, and limits.
- `drift`: compare high-risk fields between two audit JSON reports.
- `doctor`: list built-in capabilities and integrations requiring setup.

Audit captures response status/final URL, title, description, canonical count/value, robots, language, viewport, headings, links, images/alt coverage, word count, social metadata, JSON-LD blocks/types/parse failures, and hreflang. Findings carry stable IDs, severity, evidence, repair, and verification. Score covers executed static checks only and always reports scope/limitations.

Network policy rejects credentials and non-HTTP(S) schemes; resolves and rejects loopback/private/link-local/multicast/unspecified/special-use IPs; disables automatic redirects and revalidates each target; pins validated DNS; caps redirects, time, and bytes; and validates expected content types.

## Skill package

`SKILL.md` preserves Codex specialist routing, adds Rust commands, and uses observed/inferred/setup-required evidence labels. Conditional detail moves into:

- `references/audit-playbooks.md`
- `references/migration-ledger.md`

`agents/openai.yaml` keeps automatic invocation and accurately describes Rust-backed behavior. Skill carries upstream `LICENSE.txt` and `NOTICE.md`; root `THIRD_PARTY_NOTICES.md` records source and revision.

## Repository integration

- `skills-guide/overview.md`: explicit SEO category containing both requested skills.
- `README.md`: Rust runtime, evidence boundary, fifth executable skill runtime, verification commands.
- `.github/workflows/test-skills.yml`: cache, format, Clippy, and test `codex-seo`.
- `AGENTS.md`: required local `codex-seo` runtime test.
- `.codex-plugin/plugin.json`: minor version bump for new executable capability; skill count stays 55.
- `renovate.json`: existing Cargo discovery/grouping already covers nested manifest; validate without duplicate rules.

## Gates

Runtime follows workspace lint policy: Clippy `all` and `pedantic` groups at priority -1, narrow allows, `unsafe_code = deny`, broken rustdoc links denied, locked dependencies, unit tests, and fixture-driven CLI tests.

Completion requires official `skill-creator` validation, repository `skill-metadata-check`, both skill runtimes, full 21-crate gate workspace, existing skill tests, Renovate validation, live HTTPS smoke, package archive, and exact-revision isolated install test. Chronis epic closes only after every gate passes.
