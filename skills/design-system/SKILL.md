---
name: design-system
description: "Enforce design-token governance, responsive sizing, and white-label safety for multi-tenant interfaces. Use when modifying CSS, Tailwind classes, global styles, UI components, design tokens, colors, or shared visual patterns."
---

# Design System Gates

Enforces design token governance, responsive sizing, and white-label safety for multi-tenant prediction market apps.

**Trigger:** Use PROACTIVELY when writing or modifying CSS, Tailwind classes, `globals.css`, or UI components in `packages/ui/`, `apps/web/`, or `@repo/icons`. Also triggers on: "add design token", "new CSS variable", "add color", "style component", "fix styles", "review styles", "design review".

---

## Rule 1: No New Tokens Without Justification

The design system has a **minimal token set** to keep white-labelling simple. Every new CSS custom property in `globals.css` multiplies the burden for every tenant theme.

**Before adding a token, check if an existing one covers the use case:**

| Need | Existing token to use |
|------|-----------------------|
| Container/card background | `--card` or `--popover` |
| Container border/outline | `--border` |
| Primary text | `--foreground` |
| Surface/elevated bg | `--background-surface` |
| Active element fill | `--card` (light) / `--card` (dark) |
| Stroke/divider | `--border` |

**Known duplicates to avoid (from PR #1092 review):**
- `--container-fill` duplicates `--card` (both are white/dark card bg)
- `--container-stroke` duplicates `--border` (identical values)
- `--type-primary` duplicates `--foreground` (identical values)

**If you truly need a new token:**
1. Confirm no existing token serves the same **use case** (not just the same HSL value)
2. Add it to BOTH `:root` AND `.dark` in `globals.css`
3. Add the `--color-*` mapping in `@theme inline`
4. Document the Figma source in a comment (e.g., `/* Figma: container/fill */`)

---

## Rule 2: Use Responsive Units, Not Static px

Use Tailwind spacing scale, `rem`, or `em` for sizing. Static `px` values are for **very specific edge cases only** (e.g., precise icon sizes, borders, or animation targets where sub-pixel rounding matters).

**Prefer:**
```tsx
// Tailwind spacing (best)
className="h-13 w-13 gap-2 p-4 rounded-full"

// rem/em (acceptable)
style={{ maxWidth: "5rem" }}
```

**Avoid:**
```tsx
// Static px (needs justification)
className="h-[52px] w-[52px] gap-[8px] p-[16px]"
style={{ maxWidth: "80px", borderRadius: "160px" }}
```

**Exceptions where px is acceptable:**
- `1px` borders (sub-pixel rendering)
- Exact icon viewport sizes (e.g., `viewBox="0 0 24 24"`)
- `backdrop-blur` values (browser quirk)
- Values from Figma that don't map to the spacing scale (document with comment)

**Conversion reference:**
| px | Tailwind | rem |
|----|----------|-----|
| 4 | `1` | 0.25rem |
| 8 | `2` | 0.5rem |
| 12 | `3` | 0.75rem |
| 16 | `4` | 1rem |
| 20 | `5` | 1.25rem |
| 24 | `6` | 1.5rem |
| 32 | `8` | 2rem |
| 40 | `10` | 2.5rem |
| 48 | `12` | 3rem |
| 52 | `13` | 3.25rem |
| 64 | `16` | 4rem |

---

## Rule 3: Use `cn()` for Conditional Classes

Import `cn` from `@repo/ui/lib/utils` (wraps `clsx` + `tailwind-merge`). Never use template literal ternaries for className.

**Do:**
```tsx
import { cn } from "@repo/ui/lib/utils";

<button className={cn(
  "flex items-center overflow-hidden h-13",
  isActive
    ? "px-4 rounded-pill bg-card gap-2 justify-center"
    : "w-13 rounded-full bg-background-surface border border-border justify-center"
)} />
```

**Don't:**
```tsx
<button className={`flex items-center ${isActive ? "bg-card" : "bg-background-surface"}`} />
```

---

## Rule 4: Radius System

Use the defined radius tokens, not arbitrary values:

| Token | Value | Usage |
|-------|-------|-------|
| `rounded-pill` | 160px (via `--radius-pill`) | Pills, fully-rounded buttons |
| `rounded-2xl` | 1.5rem | Large cards |
| `rounded-xl` | 1.25rem | Cards |
| `rounded-lg` | 1rem | Buttons, inputs |
| `rounded-md` | 0.5rem | Small elements |
| `rounded-sm` | 0.25rem | Badges, tags |
| `rounded-full` | 9999px | Circles, avatars |

---

## Rule 5: Shared Components Over Inline

If a UI element (button variant, card, nav item) could be reused:
1. Check `packages/ui/src/` first — it likely exists
2. If not, add it to `@repo/ui` (not inline in the consuming component)
3. One-off styled elements within a component are fine if truly unique

---

## Rule 6: Color References

Always use semantic tokens via Tailwind, never raw HSL/hex:

```tsx
// Do
className="bg-card text-foreground border-border"
className="text-positive-foreground bg-positive/10"

// Don't
className="bg-white text-[#050505] border-[#e5e5e5]"
style={{ color: "hsl(158, 54%, 51%)" }}
```

---

## Rule 7: White-Label Safety Checklist

When adding any visual change, verify:
- [ ] No hardcoded colors (use tokens)
- [ ] No new tokens without checking for existing equivalents by **use case**
- [ ] Dark mode values defined if adding tokens
- [ ] Responsive units used (rem/Tailwind, not px)
- [ ] Component is in `@repo/ui` if reusable
- [ ] i18n text uses `next-intl` `t()` function

---

## Current Token Inventory (for duplicate checking)

**Backgrounds:** `background`, `background-surface`, `card`, `popover`, `overlay`
**Text:** `foreground`, `card-foreground`, `popover-foreground`, `muted-foreground`, `overlay-foreground`
**Brand:** `primary`, `secondary`, `accent`, `muted`
**States:** `success`, `warn`, `destructive`, `positive`, `negative`
**Outcomes:** `outcome-positive`, `outcome-negative`
**Borders:** `border`, `input`, `ring`
**Charts:** `chart-1` through `chart-5`

Tokens added in PR #1092 that are under review for removal/consolidation:
- `container-fill` (may merge with `card`)
- `container-stroke` (may merge with `border`)
- `type-primary` (may merge with `foreground`)
