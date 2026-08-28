# SEO audit playbooks

Use only sections relevant to requested scope. Every verdict needs evidence,
affected URLs/templates, confidence, and verification.

## Full audit

### Discovery and crawl

- Inspect robots.txt, sitemap discovery, sitemap XML shape, status, and URL
  consistency.
- Compare crawlable URLs with canonical, indexable, linked, and submitted sets.
- Find orphan candidates from analytics/log/CMS exports; no single crawler sees
  every orphan.
- Check response chains, redirect loops/hops, soft-404 evidence, duplicate URL
  patterns, parameters, faceted navigation, pagination, and crawl traps.
- For JavaScript sites, compare source HTML, rendered DOM, and browser network
  behavior.

### Index eligibility and canonicalization

- Inspect HTTP status, meta robots, X-Robots-Tag, canonical, robots blocking,
  authentication, and removal tools as separate controls.
- Check canonical reciprocity across internal links, sitemap URLs, redirects,
  alternate languages, mobile/desktop variants, and structured data URLs.
- Treat `site:` queries as clues, not complete index counts.
- Use Search Console URL Inspection or equivalent first-party evidence for
  indexation claims.

### On-page and content

- Map one primary intent and supporting questions to each page/template.
- Review title, description, primary heading, hierarchy, copy usefulness,
  authorship, update context, sources, internal links, and conversion path.
- Detect duplication with normalized content/template evidence; shared chrome is
  not automatically thin content.
- Flag unsupported claims, synthetic citations, filler, keyword stuffing, and
  pages whose value does not survive removal of template variables.
- Assess experience/expertise signals according to topic risk. High-stakes pages
  need stronger authorship, review, sourcing, and update governance.

### Structured data

- Parse all JSON-LD blocks and map types/IDs to visible page entities.
- Validate syntax with Schema.org vocabulary, then eligibility with current
  Google feature documentation. These are distinct checks.
- Ensure claims, prices, availability, ratings, dates, images, organization
  identity, and canonical URLs match visible/current evidence.
- Never promise rich results. Valid markup is not an appearance guarantee.

### Images and media

- Check meaningful alt text, decorative empty alt, dimensions/aspect ratio,
  responsive sources, modern formats, lazy loading, LCP priority, filenames, and
  image sitemap needs.
- Inspect thumbnails and social previews at real crop sizes.
- Treat generated images and IPTC AI labels according to policy and provenance;
  do not infer rights or originality.

### Performance

- Separate field data (CrUX/RUM) from lab data (Lighthouse/PageSpeed).
- Record URL/origin, form factor, percentile, sample window, and collection time.
- Diagnose LCP subparts, INP interaction traces, CLS sources, render blocking,
  caching, compression, font loading, and third parties.
- No field data means `setup required`, not a passing Core Web Vitals verdict.

### Links and authority

- Separate internal links, external links, observed backlinks, referring domains,
  mentions, anchors, lost/new links, and spam-risk hypotheses.
- Verify sampled backlinks by fetching source and target where permitted.
- Do not invent link counts, authority scores, toxicity, or competitor gaps.
- Prefer relevant editorial links and cited expertise over volume tactics.

### International

- Validate language-region codes, self-reference, return links, canonical
  alignment, HTTP versus HTML delivery, x-default, sitemap consistency, and
  locale redirects.
- Build a hreflang graph; a valid tag on one page does not prove reciprocal
  cluster integrity.
- Check translated intent and local conversion details, not metadata alone.

## Mode add-ons

### Ecommerce

- Product/category crawl paths, variants, filters, pagination, discontinued
  products, availability, prices/currency, merchant feeds, reviews, and Product/
  Offer markup consistency.
- Verify Shopping/Merchant Center data only through configured exports/APIs.

### Local and maps

- Entity name/address/phone/category/service-area consistency across site, GBP,
  directories, schema, and landing pages.
- Review location page uniqueness, local proof, hours, accessibility, parking,
  service constraints, reviews, and map intent.
- Geo-grid rank and competitor radius need configured maps evidence. Mark absent
  data `setup required`.

### Programmatic SEO

- Sample by template, data source, locale, age, traffic/index state, and edge
  cases—not only happy paths.
- Gate launch on unique utility, data quality, internal linking, canonical rules,
  empty-state handling, update ownership, and index-bloat controls.
- Roll out in cohorts with explicit stop/rollback thresholds.

### Migration and drift

- Freeze URL inventory, status/canonical/robots/hreflang/schema/title baselines,
  redirects, internal links, sitemaps, analytics tags, and top-query/page data.
- Compare before/after at template and high-value URL levels.
- Monitor response/index/traffic/conversion deltas with deployment timestamps.
- Correlation after release is investigation evidence, not automatic causation.

### Competitors, clusters, and content planning

- Freeze query set, market, language, device, and collection time.
- Cluster by observed result overlap and intent; semantic similarity alone does
  not prove one SERP intent.
- Represent competitor strengths fairly. Distinguish sourced facts from proposed
  positioning.
- Briefs need audience, intent, angle, evidence requirements, outline, internal
  links, schema candidates, conversion action, and acceptance checks.

### Search experience optimization

- Inspect result promise, landing-page match, above-fold comprehension, trust,
  accessibility, task completion, and conversion friction together.
- Measure search impressions/clicks separately from on-site engagement and
  conversions. Avoid optimizing click-through into a poor task outcome.

### AI-search/GEO

- Make key facts answer-first, locally complete, attributable, and easy to quote
  without losing qualifiers.
- Use stable headings, explicit entities/relationships, primary sources, dates,
  authors/reviewers, and concise tables where comparison benefits.
- Test citation/mention presence on named platforms only with current observed
  prompts and preserve prompt, locale, model/product, and capture time.
- `llms.txt` may aid discovery for willing consumers; never describe it as a
  universal indexing or ranking control.

## Evidence quality

Preferred order:

1. first-party server/CMS/search/analytics/conversion evidence;
2. live response and rendered browser evidence;
3. official platform documentation and validators;
4. reputable third-party datasets with methodology and timestamp;
5. manual observation;
6. inference labeled with confidence.

Cross-check high-impact findings with two independent evidence types when
possible.
