# Claude SEO migration ledger

Source: [AgriciDaniel/claude-seo](https://github.com/AgriciDaniel/claude-seo),
revision `a1480c7e590b16001bd9dc1627eacdcd44d580f9`, release `2.2.5`.

This ledger records where upstream capability lives after Codex migration.

## Status meanings

- **Rust core** — deterministic, local implementation in `scripts/`.
- **Workflow** — Codex instructions, browser use, or installed specialist skill.
- **Setup required** — capability depends on unavailable credentials, connector,
  commercial API, or live platform data.

## Capability map

| Upstream skill | Codex destination | Status |
| --- | --- | --- |
| `seo` | `$codex-seo` orchestration plus `$seo` when installed | Workflow |
| `seo-audit` | audit playbook, specialist routing, Rust `audit` | Rust core + workflow |
| `seo-page` | page playbook and Rust `audit` | Rust core + workflow |
| `seo-technical` | technical playbook and installed specialist | Workflow |
| `seo-content` | content playbook and installed specialist | Workflow |
| `seo-schema` | JSON-LD detection in Rust, generation or validation in specialist | Rust core + workflow |
| `seo-sitemap` | Rust `sitemap` and installed specialist | Rust core + workflow |
| `seo-hreflang` | hreflang playbook and installed specialist | Workflow |
| `seo-drift` | Rust `drift` and installed specialist | Rust core + workflow |
| `seo-images` | image signals in Rust and installed specialist | Rust core + workflow |
| `seo-performance` | specialist plus browser or field-data tools | Setup required |
| `seo-google` | Search Console, GA4, and CrUX connectors | Setup required |
| `seo-dataforseo` | DataForSEO connector | Setup required |
| `seo-backlinks` | backlink APIs or supplied export | Setup required |
| `seo-firecrawl` | Firecrawl connector | Setup required |
| `seo-visual` | browser screenshots and visual specialist | Workflow |
| `seo-image-gen` | image-generation tool and specialist | Workflow |
| `seo-maps` | maps or geo-grid provider | Setup required |
| `seo-local` | local playbook and installed specialist | Workflow |
| `seo-ecommerce` | ecommerce playbook and installed specialist | Workflow |
| `seo-cluster` | live SERP evidence and installed specialist | Setup required |
| `seo-competitor-pages` | competitor playbook and installed specialist | Workflow |
| `seo-plan` | planning playbook and installed specialist | Workflow |
| `seo-programmatic` | programmatic playbook and installed specialist | Workflow |
| `seo-sxo` | browser or live SERP evidence and installed specialist | Setup required |
| `seo-geo` | GEO playbook and installed specialist | Workflow |
| `seo-flow` | evidence labels and phased workflow | Workflow |

## Intentional changes

- No autonomous search-engine submission or indexing mutations.
- No bundled PDF, spreadsheet, or document generators.
- No bundled browser runtime.
- No fabricated API, rank, backlink, performance, or traffic results.
- No unrestricted recursive crawler.
- No automatic third-party installation.

Those boundaries keep default execution deterministic, auditable, and safe.
