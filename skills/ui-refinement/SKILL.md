---
name: ui-refinement
description: Review and refine a website or desktop app page by page, recording design decisions and enforcing contribution gates with visual and runtime evidence.
---

# Review the interface

Apply this method to the requested product and scope. It is independent of framework, rendering engine and operating system. Discover the repository's instructions, design system, existing decisions, current changes and actual verification commands. Preserve unrelated work. A skill is not a permission grant; commits, publication and external writes require their usual authority.

## Use with the ritual

For a planned interface change, use `ritual` or `ritual-full` as the delivery router and this method as its UI specialist. Invoke both by name, for example: “Use ritual-full with ui-refinement to refine the account and journal pages.” Read the selected ritual and its current step first. Follow its preflight, Chronis epic, handoffs and gates; this method neither starts a second flow nor grants approval. If the selected ritual is unavailable, report that and its [installation guide](https://github.com/decebal/decebal-claude-skills/blob/main/docs/ritual-zero-to-hero.md); do not pretend the full flow ran. A standalone UI review can use this method without a ritual.

Add UI evidence to the ritual's existing artifacts instead of creating a competing plan or tracker:

| Ritual step | UI contribution |
| --- | --- |
| Think | Define the user journey, scope, affected page families and unknown states in the brief. |
| Spec | Put the decision, representative evidence, shared patterns, exceptions and acceptance criteria in or linked from the design. For full ritual, retain its Semantic ISA and independent design reviews. |
| Plan | Map each affected page and state to its source and verification; add browser, accessibility and regression checks to the existing testing trophy and dependency graph. |
| Build | Claim ready work through the ritual, capture before and after for the tested build, implement decisions, and run the relevant story-level checks. |
| Review | Give the fresh-context reviewer the design, diff, page inventory, decisions and evidence; exercise the journey and verify untested states are not called passes. |
| Ship | Attach the existing evidence to the ritual's PR proof and run the actual contribution and release gates. |
| Reflect | Record UI drift, gate failures and uncovered states in the ritual report. |

The ritual's approval, review and drift rules remain authoritative. Do not change an approved design to append implementation results; put those in the plan, review or linked decision record according to the ritual.

For an authorized fix handoff, include the tested build and environment, route and viewport, starting state, exact actions and test data, observed and expected behavior, impact and evidence. Name affected sources, shared consumers, existing gates, worktree ownership, uncommitted work and permitted actions. State what remains unverified; require the recipient to reproduce the journey and retain source-bound screenshots. Do not send secrets or private customer data.

## Establish a baseline

Inventory every reachable page and material state in scope, including entry points, empty, loading, error and populated states, responsive sizes, themes and input modes where applicable. Record exclusions and access blockers. Map each page to its user goal, primary action, source implementation and shared patterns. Start the actual application using a bounded, owned process; inspect the rendered product, not source alone. Record build identity, environment, viewport, state and date for every capture. Never equate a remote preview with a local checkout without evidence binding them.

Capture comparable screenshots for representative states and widths, then exercise the critical journey, keyboard focus and recovery paths. Check hierarchy, understandable labels, content, navigation, spacing, typography, contrast, responsive reflow, interaction feedback and accessibility. Record observed defects with trigger, user consequence, location and evidence; mark unobserved states unverified. Consider how a local fix affects every consumer of a shared component.

For efficiency, discover routes and shared components once, then batch deterministic presence, reflow and broken-link checks across affected pages. Inspect representative shared patterns and each distinct page or state that can change the user's decision or journey; record per-page results and exceptions. Run focused checks while iterating and whole-product gates once at the ritual's integration boundary. A batch result never substitutes for visual judgment or proves an unvisited state.

For shared navigation and footer changes, inventory every page family and rendering path that uses them. At narrow and wide sizes, check visible labels, link destinations, reflow and keyboard order and focus, including a journey through the actual links. Capture representative header and footer states and record which families were exercised. If a regression gate checks source structure alone, keep it and add rendered checks where the project can run them; do not treat source presence as proof of working navigation.

## Decide and refine

For each page, record the intended user outcome, alternatives, decision and rationale. Link before/after screenshots and actual source examples. Reuse existing decisions and tokens when suitable; document deliberate exceptions and their scope. Keep product-specific values in the product, with portable workflow instructions here. Update decisions when superseded instead of leaving conflicting rules. Do not present a proposed design as accepted without an authorized decision.

Freeze deterministic checks and their inputs before measuring any experiment. Use a project adapter for build, capture and evaluation. When autoresearch-rs is available, freeze its manifest, evaluator, environment, candidate scope and objective, run a baseline, change one hypothesis per candidate, and retain the actual keep/discard and verification evidence. Its hard gates must pass independently of the score. Screenshot similarity, accessibility samples and numerical scores do not establish usability or visual quality: require human judgment and actual task completion as separate checks. If a Git checkout, runnable app or required evaluator is missing, record the blocker and do not claim a completed experiment.

## Make future changes accountable

Maintain an indexed design decision record in the product repository, with source links and screenshot paths, approved examples, rule owner, exceptions and review date. Define contribution gates with an explicit trigger, command or reviewer procedure, expected result and failure behavior. Gate changed and affected pages on: usable primary journey; consistency with applicable decisions; clear labels and hierarchy; responsive or window-size reflow; keyboard/focus and relevant accessibility; screenshots tied to the tested build; and absence of unexplained regressions. A missing or unrun check is unverified, not a pass. Verify the real enforcement path and its scope with rejected and accepted contributions. A local Git hook blocks only where installed; a merge gate blocks only if repository policy requires it. Distinguish evidence-presence checks from human approval of clarity and consistency. Do not make pixel equality a substitute for design review.

## Finish with evidence

Reopen every affected page, repeat the relevant journeys and compare matching captures. Report actual commands and results, decisions and exceptions, experiment baseline and candidate outcomes, and remaining unverified states. Preserve the evidence in indexed repository docs. Never claim complete page coverage from a representative sample or a green score.
