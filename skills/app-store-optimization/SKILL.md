---
name: app-store-optimization
description: Research, validate, and improve Apple App Store and Google Play listings with current platform rules, evidence-led competitor analysis, metadata optimization, localization, review synthesis, visual planning, and controlled experiments. Use for ASO audits, store metadata, app keywords, product-page conversion, listing experiments, launches, and updates. Includes a Rust metadata and experiment validator.
---

# App Store Optimization

Improve discoverability and product-page conversion without inventing market
data. Separate platform compliance, observed evidence, hypotheses, and measured
results.

## Compose existing skills

Use smallest useful combination available in current Codex context:

- `$codex-seo` for evidence collection, current-source checks, and priorities;
- `$seo-content` for positioning, message hierarchy, and readable metadata;
- `$seo-competitor-pages` for fair competitor framing and differentiation;
- `$seo-images` for icon, screenshot, and preview asset review;
- `$seo-plan` for sequenced rollout;
- `$skill-autoresearch` for bounded metadata iteration against frozen evals.

Missing specialists do not block workflow. Apply their concern directly and mark
unavailable data or tooling.

## Establish scope

Record:

- platform: Apple, Google Play, or both;
- app identifier, market, locale, category, lifecycle stage;
- current listing and proposed fields;
- product promise, audience, differentiators, and prohibited claims;
- available first-party analytics, keyword/rank data, reviews, competitors, and
  prior experiment results.

Do not assign search volume, difficulty, rank, downloads, category benchmarks,
or conversion lift without observed source data. A generated keyword is a
hypothesis, not measured opportunity.

## Verify current rules

Read [references/platform-rules.md](references/platform-rules.md). Platform
policies change: browse linked official Apple and Google sources before final
compliance claim. Update validator only when authoritative rules changed and add
boundary tests first.

## Run Rust validator

Runner ships at `scripts/Cargo.toml`. Resolve path relative to installed
`SKILL.md` and use absolute manifest path. Rust/Cargo is required. First build
uses locked `serde_json` dependencies from local Cargo cache or crates.io.

Prepare one JSON file per platform and locale:

```json
{
  "platform": "apple",
  "locale": "en-US",
  "name": "TaskFlow",
  "subtitle": "Plan less. Finish more.",
  "promotional_text": "New shared planning for focused teams.",
  "description": "Full product description",
  "keywords": "tasks,planner,focus,team",
  "whats_new": "Improved shared planning."
}
```

Validate:

```bash
cargo run --quiet --locked --manifest-path <aso-manifest> -- validate --input listing.json
```

Exit `0` means required fields and enforced limits pass. Exit `2` means invalid
listing. Rust report remains structural evidence, not App Review approval.

## Research and optimize

1. Collect current store listings, reviews, visual assets, and first-party data.
   Record URL/app ID, locale, region, and capture time.
2. Map user language to verified features and benefits. Exclude competitor marks,
   irrelevant terms, and unsupported superlatives.
3. Draft variants within platform limits. Preserve natural language and one clear
   promise; do not stuff repeated keywords.
4. Validate every locale with Rust. Apple keyword budget uses UTF-8 bytes, so
   non-ASCII metadata can consume more than one byte per character.
5. Review first three screenshots as a narrative: audience/problem, core outcome,
   differentiator or proof. Treat visual changes as separate experiment variables.
6. Prioritize by observed gap, expected mechanism, effort, and measurement plan.

## Experiments

Change one major variable per test. Define hypothesis, primary metric, guardrails,
allocation, minimum run condition, and stop rule before launch. Analyze completed
two-variant counts:

```bash
cargo run --quiet --locked --manifest-path <aso-manifest> -- experiment \
  --control-conversions 120 --control-visitors 2000 \
  --variant-conversions 150 --variant-visitors 2000 --alpha 0.05
```

Rust output provides two-proportion z-test snapshot. Account for store experiment
method, sequential peeking, seasonality, multiple comparisons, and traffic mix
before rollout decision.

## Output

Return:

1. scope, sources, locale, and collection time;
2. structural validation report;
3. observed findings versus hypotheses;
4. before/after metadata with character or byte counts;
5. visual and localization plan;
6. experiment backlog with primary metrics and guardrails;
7. missing evidence, risks, owners, and next measurement.

This skill is adapted under MIT; see [NOTICE.md](NOTICE.md) and
[LICENSE.txt](LICENSE.txt).
