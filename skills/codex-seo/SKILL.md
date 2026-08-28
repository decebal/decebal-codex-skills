---
name: codex-seo
description: Coordinate evidence-led SEO audits, page reviews, technical checks, content analysis, schema work, GEO, local SEO, and planning in Codex. Use when asked to audit or improve search visibility, choose among installed seo specialist skills, or produce a prioritized SEO action plan without fabricating crawl, ranking, performance, or traffic data.
---

# Codex SEO

Route SEO work to available specialist skills, then combine evidence into one
decision-ready result. This skill is an original compatibility layer; it does
not bundle any third-party SEO suite.

## Route first

Inspect skills available in current Codex context.

1. If `$seo` exists, use it as primary orchestrator.
2. For a narrow request, invoke matching installed specialist directly. Common
   routes include `$seo-technical`, `$seo-content`, `$seo-schema`, `$seo-page`,
   `$seo-performance`, `$seo-images`, `$seo-sitemap`, `$seo-geo`, `$seo-local`,
   `$seo-hreflang`, `$seo-backlinks`, and `$seo-plan`.
3. If requested specialist is missing, continue with fallback below. State
   missing capability once; do not pretend its API, crawler, or dataset ran.

Do not install, execute, or copy code from an external SEO repository unless
user explicitly requests that separate action and its license permits it.

## Scope audit

Clarify target only when repository or URL cannot be inferred. Choose smallest
useful mode:

- **Page review:** one URL or rendered page.
- **Technical review:** crawl/indexing, canonical, robots, sitemap, redirects,
  status codes, rendering, performance evidence.
- **Content review:** intent coverage, structure, originality, entities, internal
  links, and passage-level citability.
- **Structured data:** detected JSON-LD and schema eligibility; validation still
  requires an authoritative validator.
- **Plan:** prioritized actions from already-collected evidence.

For broad audits, start with page and technical evidence. Add specialist areas
only when target type and available data justify them.

## Fallback workflow

When no SEO suite is installed:

1. Read user-provided repository files or open requested public pages.
2. Capture direct evidence: response behavior, rendered headings and copy,
   metadata, links, robots directives, canonical, sitemap references, and
   structured data visible in source or browser state.
3. Browse current search-engine documentation for claims that may have changed.
   Prefer official Google, Bing, Schema.org, and web-platform sources.
4. Separate findings into `observed`, `inferred`, and `not checked`.
5. Rank fixes by impact, confidence, effort, and dependency order.

Never invent keyword volume, rankings, backlinks, traffic, Core Web Vitals field
data, index coverage, or competitor performance. Mark each as `setup required`
unless direct evidence exists.

## Output contract

Return:

1. target and audit mode;
2. evidence sources and collection time;
3. critical findings with exact page, file, selector, header, or artifact;
4. prioritized actions with expected outcome and verification step;
5. limits, unmeasured areas, and required integrations.

Avoid generic SEO checklists when evidence supports fewer, sharper findings.
