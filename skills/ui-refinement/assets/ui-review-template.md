# UI refinement review

Use this template within the ritual's existing brief, design, plan, review and PR proof. Remove inapplicable rows with a reason; retain untested rows as unverified. One review may cover a shared component, but list every affected page family and the states actually inspected.

## Scope and baseline

- Product, audience, jobs and design direction:
- Build identity, URL or app, environment and date:
- Reviewer, input methods, assistive technology, browsers or devices:
- Target standard and level: WCAG 2.2 AA by default for web content, unless the project specifies another applicable baseline. Record desktop accessibility requirements separately.
- Existing design decisions, tokens, components and exclusions:
- Journey and success criteria:

| Route or page family | Goal and primary action | State and viewport | Source and shared consumers | Status and evidence |
| --- | --- | --- | --- | --- |
|  |  |  |  | pass / fail / unverified; screenshot and build |

Include entry, empty, loading, populated, error, disabled and recovery states where relevant; record mobile and desktop, keyboard and touch, and available themes. Compare screenshots of the same state and viewport. Do not mark a route reviewed merely because it shares a template.

## Accessibility audit

Use the project's accessibility tooling on the rendered, interactive state. Record tool and version, configuration, routes, viewport, actual command and output. Follow with manual keyboard and assistive-technology checks. Automated results alone do not establish conformance. For each finding, record the WCAG criterion, observed behavior, impact, reproduction, source, evidence, status and retest.

| Check | Procedure and evidence | Result |
| --- | --- | --- |
| Semantics and names | Inspect headings, landmarks, image alternatives, control roles, names, states, labels, errors and status messages. |  |
| Keyboard and focus | Complete each journey using keyboard; inspect focus order, visible focus, traps, dialogs, disclosure and author-created overlays obscuring focus. |  |
| Contrast and non-color cues | Measure actual text and adjacent colors, including interactive states; verify required information is not conveyed by color alone. |  |
| Reflow and zoom | Exercise 200% text resize and 320 CSS-pixel reflow; document standard exceptions and any lost content or functionality. |  |
| Pointer and input | Check target size and spacing under applicable WCAG 2.2 criteria and exceptions, touch, alternative inputs and cancellation where relevant. |  |
| Media and motion | Check alternatives and controls for applicable media, flashing, reduced motion and pause or stop controls where required. |  |
| Forms and errors | Exercise labels, instructions, errors, correction, authentication and recovery with keyboard and assistive technology. |  |

Reference: [WCAG 2.2](https://www.w3.org/TR/WCAG22/). Apply actual success-criterion scope and exceptions. For example, normal text contrast is generally 4.5:1 and large text 3:1; required non-text component and state contrast is generally 3:1. A 44-pixel target can be a product goal; WCAG 2.2 AA target-size minimum is 24 CSS pixels subject to exceptions. Do not claim complete WCAG conformance without evaluating all applicable criteria, complete processes and conformance requirements.

## Design and taste review

Review the rendered product against its actual audience, intended task and established visual direction. Assess hierarchy, first-screen meaning, content density, readable typography, rhythm, alignment, imagery, responsive composition and meaningful interaction feedback. Identify generic or repetitive patterns when they weaken the product; retain patterns that serve the user. Use reference sites as inspiration with links and observations, not as a license to copy branding or override product decisions. Preserve content truth and avoid inventing imagery or claims.

| Page or state | Observed issue and consequence | Decision, alternative and rationale | Source and before / after evidence | Result |
| --- | --- | --- | --- |
|  |  |  |  | pass / fail / unverified |

Inventory controls and text links across page families, including generated, static, guide and printable screen pages. Record purpose and semantic role, approved variants, action hierarchy, spacing at narrow and wide widths, keyboard and touch behavior, and consistent inline-link treatment. Check that actions are necessary and separated; document deliberate exceptions. Preserve print-specific layouts and verify print output separately.

Taste reference: [design-taste-frontend](https://github.com/Leonxlnx/taste-skill/blob/main/skills/taste-skill/SKILL.md). Apply relevant design judgments to the existing product and stack; do not require its starter, scripts, design tokens or randomly selected layout. Record unavailable tools or skills rather than claiming they ran.

## Contribution gates and follow-through

| Invariant / acceptance criterion | Trigger and actual check | Passing and failing evidence | Enforcement surface and coverage | Owner / exception |
| --- | --- | --- | --- | --- |
|  |  |  | local hook / CI / required merge rule / human review |  |

Bind review records and screenshots to the tested source and build. Demonstrate gates reject a representative violation and accept a valid change. Preserve existing hooks and release checks. State which paths can block a contribution and who can bypass them; local hooks and advisory CI do not guarantee enforcement for every contributor. Human review decides control necessity, visual quality and clarity; automated checks cover only the assertions they execute.

- Decisions, alternatives, approved code examples and exceptions:
- Findings with impact, severity and reproduction:
- Commands, tool versions, outcomes and unverified checks:
- Remaining pages and states, blockers and retest owner:
- Final journey result and evidence links:
