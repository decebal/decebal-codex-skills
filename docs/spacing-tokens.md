# CSS Spacing & Design Tokens

Framework-agnostic (Svelte / React / Vue / plain CSS). Without a shared spacing
scale, every component picks its own raw pixel values and the UI drifts. One real
project measured **0% token adoption across 10 settings components — 600+ raw
`px` values**; after adopting this scale, files reached 60–83% adoption and the
visual consistency was immediate.

## Spacing scale

Define once, at `:root`:

```css
:root {
  --sp-1: 2px;   /* hairline gaps */
  --sp-2: 4px;   /* tight */
  --sp-3: 6px;   /* compact */
  --sp-4: 8px;   /* default gap */
  --sp-5: 10px;  /* medium */
  --sp-6: 12px;  /* section spacing */
  --sp-7: 14px;
  --sp-8: 16px;  /* large */
  --sp-9: 20px;  /* XL */
  --sp-10: 24px; /* 2XL */
  --sp-11: 32px; /* page-level */
  --sp-12: 40px; /* maximum */
}
```

## Component patterns

| Element | Token |
|---------|-------|
| Card inner padding | `--sp-6` |
| Form-field vertical rhythm | `--sp-4` |
| Between sections | `--sp-9` |
| Grid: between cards | `--sp-6` |
| Grid: between list items | `--sp-3` |
| Modal body padding | `--sp-8` |

## Migration cheat sheet

| Raw value | Token |
|-----------|-------|
| 4px | `var(--sp-2)` |
| 8px | `var(--sp-4)` |
| 12px | `var(--sp-6)` |
| 16px | `var(--sp-8)` |
| 24px | `var(--sp-10)` |

## Enforcement

Measure adoption per file (raw px is the anti-pattern, tokens are the goal):

```bash
grep -oE '[0-9]+px' Component.svelte | wc -l              # raw (should trend to 0)
grep -oE 'var\(--sp-[0-9]+\)' Component.svelte | wc -l    # tokens (should grow)
```

Migrate incrementally: convert the highest-traffic components first, track the
ratio, and treat a new raw `px` in review as a regression once a file is on tokens.
