---
name: codex-seo
description: Audit, plan, and improve technical SEO, on-page content, structured data, sitemaps, hreflang, local or ecommerce visibility, and AI-search citability using evidence-first Codex workflows and a safe Rust static-analysis CLI. Use for SEO audits, page reviews, traffic or ranking investigations, content plans, schema, crawlability, indexation, migration drift, competitor research, and GEO. Never invent search, analytics, backlink, or performance data.
---

# Codex SEO

Run evidence-led SEO work in Codex. Combine installed specialist skills with a
safe Rust static-analysis core.

## Evidence rules

Label every material claim:

- **Observed** — directly present in fetched HTML, headers, structured data,
  supplied exports, screenshots, or tool output.
- **Inferred** — reasoned from observed evidence. State assumptions.
- **Setup required** — needs credentials, a connector, a browser session, field
  performance data, search results, or another unavailable source.

Never invent rankings, impressions, clicks, backlinks, Core Web Vitals, crawl
coverage, or competitor metrics. Never present static inspection as proof of
indexation or real-user performance.

## Compose installed skills

Use `$seo` as broad orchestrator when installed. Route focused work to matching
`$seo-*` specialists such as technical, content, schema, sitemap, hreflang,
performance, local, ecommerce, competitor, programmatic, SXO, or GEO.

Use this skill's Rust CLI for deterministic page, sitemap, and drift checks.
Keep specialist reasoning and live-data collection in Codex. Do not install
third-party suites unless user asks.

## Choose mode

Read [references/audit-playbooks.md](references/audit-playbooks.md) only for
requested mode:

- full audit
- single-page review
- technical audit
- content or on-page review
- schema, sitemap, or hreflang review
- local or ecommerce review
- competitor, topic cluster, or programmatic plan
- migration drift
- AI-search or GEO review

Read [references/migration-ledger.md](references/migration-ledger.md) when
checking upstream compatibility or deciding whether work belongs in Rust,
specialist skills, or setup-required integrations.

## Collect evidence

1. Confirm target, scope, locale, audience, business goal, and comparison
   baseline when relevant.
2. Fetch only URLs user placed in scope. For local files, pass explicit
   `--base-url` when relative links matter.
3. Use browser, current official documentation, and connected APIs when live
   evidence matters.
4. Preserve source URLs and collection time.
5. Treat robots directives, canonical tags, schema, and sitemap entries as
   signals, not guarantees of search-engine behavior.

## Run Rust core

From repository root:

```bash
cargo run --quiet --locked --manifest-path skills/codex-seo/scripts/Cargo.toml -- audit --input https://example.com --format markdown
cargo run --quiet --locked --manifest-path skills/codex-seo/scripts/Cargo.toml -- audit --input ./page.html --base-url https://example.com/page --format json
cargo run --quiet --locked --manifest-path skills/codex-seo/scripts/Cargo.toml -- sitemap --input https://example.com/sitemap.xml --format markdown
cargo run --quiet --locked --manifest-path skills/codex-seo/scripts/Cargo.toml -- drift --baseline before.json --current after.json --format markdown
cargo run --quiet --locked --manifest-path skills/codex-seo/scripts/Cargo.toml -- doctor
```

Installed skill path may differ. Resolve `scripts/Cargo.toml` relative to this
`SKILL.md`.

Exit code `0` means command completed without high or critical findings. Exit
code `2` means audit completed and found high or critical findings.

Rust core:

- reads local HTML and XML or public HTTP(S) targets;
- blocks loopback, private, link-local, multicast, documentation, and other
  special-use network destinations;
- revalidates each redirect and pins validated DNS results;
- disables ambient proxies and credential forwarding;
- caps redirects, response size, and request time;
- audits titles, descriptions, headings, robots, canonicals, links, images,
  language, Open Graph, and JSON-LD;
- validates sitemap structure and URL scope;
- compares JSON audit snapshots for migration drift.

Rust core does not execute JavaScript, authenticate to analytics or search
platforms, crawl arbitrary sites recursively, measure field performance, or
mutate external systems.

## Analyze mechanisms

Prioritize causes over symptom counts:

- discovery and crawl path;
- indexability and canonical consistency;
- rendering and content availability;
- intent alignment and information gain;
- internal-link graph and orphan risk;
- structured-data eligibility;
- localization and hreflang graph integrity;
- media delivery and layout stability;
- conversion path and search-experience quality;
- citation-ready passages for AI answer systems.

Explain dependencies. Example: a missing canonical plus conflicting sitemap URL
is one consolidation problem, not two unrelated checklist items.

## Return result

Use this structure:

1. **Scope and evidence** — targets, tools, date, inaccessible sources.
2. **Executive diagnosis** — strongest mechanisms and business impact.
3. **Findings** — evidence, severity, confidence, affected URLs, fix, validation.
4. **Prioritized plan** — now, next, later; owner and dependency when known.
5. **Setup required** — exact connector, credential, export, or measurement
   needed for unsupported claims.
6. **Verification** — command, re-crawl, browser check, or platform report that
   proves each fix.

Keep recommendations specific enough to implement. Include exact tags, URLs,
schema types, internal-link targets, or content changes when evidence supports
them.

## Attribution

Workflow adapted from
[AgriciDaniel/claude-seo](https://github.com/AgriciDaniel/claude-seo).
See [NOTICE.md](NOTICE.md) and [LICENSE.txt](LICENSE.txt).
