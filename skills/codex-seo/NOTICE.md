# Notice

`codex-seo` adapts workflow concepts from
[AgriciDaniel/claude-seo](https://github.com/AgriciDaniel/claude-seo) at revision
`a1480c7e590b16001bd9dc1627eacdcd44d580f9` (release `2.2.5`).

Upstream copyright:

> Copyright (c) 2026 agricidaniel

Material changes:

- consolidated 25 upstream skills into one Codex entrypoint that composes
  installed SEO specialists;
- replaced 53 Python helper scripts with one Rust static-analysis CLI;
- added SSRF controls, redirect revalidation, DNS pinning, response caps, and
  proxy isolation;
- preserved specialist routing for technical, content, schema, sitemap,
  hreflang, local, ecommerce, competitor, programmatic, SXO, and GEO work;
- added explicit observed, inferred, and setup-required evidence labels;
- removed autonomous external mutations and unsupported live-data fallbacks;
- recorded capability parity in `references/migration-ledger.md`.

Distributed under upstream MIT license. See `LICENSE.txt`.
