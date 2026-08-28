---
name: codex-prd
description: "Generate a Product Requirements Document (PRD) for Codex task orchestration. Create PRDs with user stories, two-tier quality gates, and a testing-trophy plan that the codex-beads skill can convert into tracked work. Use when asked to create a PRD, plan a feature, write requirements, or spec out work."
---

# Codex PRD Generator

Create detailed Product Requirements Documents optimized for Codex execution.

---

## The Job

1. Receive a feature description from the user
2. Ask 3-5 essential clarifying questions (with lettered options) - one set at a time
3. **Always ask about quality gates** — classify into epic-level vs. story-level
4. After each answer, ask follow-up questions if needed (adaptive exploration)
5. Generate a structured PRD when you have enough context
6. Output the PRD wrapped in `[PRD]...[/PRD]` markers for TUI parsing

**Important:** Do NOT start implementing. Just create the PRD.

---

## Step 1: Clarifying Questions (Iterative)

Ask questions one set at a time. Each answer should inform your next questions. Focus on:

- **Problem/Goal:** What problem does this solve?
- **Core Functionality:** What are the key actions?
- **Scope/Boundaries:** What should it NOT do?
- **Success Criteria:** How do we know it's done?
- **Integration:** How does it fit with existing features?
- **Quality Gates:** What commands validate success, and which tier? (REQUIRED)
- **Testing Trophy:** What test layers are needed? (unit, contract, integration, frontend, E2E) (REQUIRED)

### Format Questions Like This:

```
1. What is the primary goal of this feature?
   A. Improve user onboarding experience
   B. Increase user retention
   C. Reduce support burden
   D. Other: [please specify]

2. Who is the target user?
   A. New users only
   B. Existing users only
   C. All users
   D. Admin users only
```

This lets users respond with "1A, 2C" for quick iteration.

### Quality Gates Question (REQUIRED)

Always ask about quality gates with **two-tier classification**:

```
What commands validate the codebase? These run ONCE when all stories are done (epic-level):
   A. task ci (go-task runner)
   B. make ci
   C. bun typecheck && bun lint
   D. cargo test
   E. Other: [specify your commands]

For individual stories, what story-specific checks apply?
   A. UI stories: verify in browser using `$codex-browser`
   B. UI stories: Playwright e2e test for that story
   C. Backend stories: curl/test the specific endpoint
   D. Other: [specify]
```

**Why two tiers?** Running `bun typecheck` after every single story wastes agent context and time. Intermediate stories may have temporary type errors that the next story fixes. General checks run once at the end.

### Adaptive Questioning

After each response, decide whether to:
- Ask follow-up questions (if answers reveal complexity)
- Ask about a new aspect (if current area is clear)
- Generate the PRD (if you have enough context)

Typically 2-4 rounds of questions are needed.

### Testing Trophy Question (REQUIRED)

Always ask about test coverage expectations:

```
What test layers does this epic need?
   A. Unit only (pure logic, projections, state machines)
   B. Unit + Contract (data shapes across IPC boundary)
   C. Unit + Contract + Integration (cross-service flows)
   D. Full trophy: Unit + Contract + Integration + Frontend + E2E
   E. Other: [specify]

Which behaviors are "block merge" critical?
   A. Auto-completion / background logic (bugs are invisible to users)
   B. Data filtering / query correctness (wrong data shown)
   C. Orchestration (wrong things triggered)
   D. All of the above
   E. Other: [specify]
```

Use the answers to populate the Testing Trophy section in the PRD.

---

## Step 2: PRD Structure

Generate the PRD with these sections:

### 1. Introduction/Overview
Brief description of the feature and the problem it solves.

### 2. Goals
Specific, measurable objectives (bullet list).

### 3. Quality Gates (Two Tiers)

**CRITICAL:** Split gates into two tiers. The conversion tool (the `codex-beads` skill) reads this section directly.

```markdown
## Quality Gates

### Epic-Level (run once on epic completion)
General codebase checks. These validate the whole project and run ONCE when all stories are done:
- `task ci` — full CI pipeline (typecheck + lint + test)
- `bun typecheck` — type checking
- `bun lint` — linting

### Story-Level (checked per story)
Criteria checked on specific stories where relevant:
- **UI stories:** Verify in browser using `$codex-browser`
- **Backend stories:** Verify endpoint returns expected response
- **Migration stories:** Verify column/table exists after migration
```

### 4. User Stories

Each story needs:
- **Title:** Short descriptive name
- **Type tag:** `[Schema]`, `[Backend]`, `[UI]`, `[Integration]` — tells the conversion tool which story-level gates apply
- **Description:** "As a [user], I want [feature] so that [benefit]"
- **Acceptance Criteria:** Verifiable `- [ ]` checkboxes the agent marks as done
- **Mark-each-item instruction:** Reminder to verify and check off each item

Each story should be small enough to implement in one focused AI agent session.

**Format:**
```markdown
### US-001: [Title] [Schema]
**Description:** As a [user], I want [feature] so that [benefit].

**Acceptance Criteria:**
- [ ] Specific verifiable criterion
- [ ] Another criterion
- [ ] (story-level gate if applicable)

Mark each item [x] as you complete it. Only close when all are checked.
```

**Important — writing verifiable criteria:**

Every criterion must be something the agent can concretely verify:

**Good (agent can check these):**
- `- [ ] File apps/core/src/new_module.rs exists and compiles`
- `- [ ] GET /api/v1/events returns 200 with JSON body`
- `- [ ] Column investorType added with default 'cold' (check migration file)`
- `- [ ] Component renders without errors (verify in browser)`

**Bad (agent cannot verify these):**
- `- [ ] Works correctly`
- `- [ ] Good UX`
- `- [ ] Handles edge cases`
- `- [ ] Performant`

**Note:** Do NOT include epic-level gates (typecheck, lint, ci) in individual story criteria — they are defined once in the Quality Gates section and run on epic completion only. Story-level gates (browser verification, endpoint testing) DO belong in the relevant stories.

### 5. Testing Trophy

**REQUIRED for every epic.** Define the expected test coverage across all layers BEFORE implementation begins. This prevents shipping features with test debt.

The testing trophy prioritizes integration and contract tests over unit tests and E2E:

```
         ╭───────╮
         │  E2E  │  Few, slow, high confidence
      ╭──┴───────┴──╮
      │ Integration  │  Cross-boundary flows
   ╭──┴─────────────┴──╮
   │  Contract/Handler  │  Data shapes, API boundaries
╭──┴───────────────────┴──╮
│     Unit (logic only)    │  Pure functions, state machines
╰──────────────────────────╯
```

**For each layer, list specific tests the epic requires:**

```markdown
## Testing Trophy

### Unit Tests
- [ ] [Projection] TaskRunProjection handles Started/Completed events (N tests)
- [ ] [Store] get_outputs filters deliverable steps only
- [ ] [Logic] auto-completion derives correct status from sibling runs

### Contract Tests
- [ ] [Handler] list_task_runs returns sorted TaskRunSummary array
- [ ] [Handler] run_job returns task_run_id, creates workflows
- [ ] [Handler] get_task_run_outputs filters is_deliverable == true
- [ ] [Frontend API] listTaskRuns/runJob/getTaskRunOutputs match handler shapes

### Integration Tests
- [ ] [Service] run_job creates TaskRun + triggers linked workflows
- [ ] [Flow] workflow completion → auto-completes parent TaskRun

### Frontend Tests
- [ ] [Component] Empty state shows Run Job CTA
- [ ] [Component] Accordion expand loads outputs
- [ ] [Component] Approve/Reject actions invoke correct commands
```

**TDD-First Rules:**

Every story follows test-driven development. Tests are written BEFORE or ALONGSIDE implementation, never after.

- **Every user story** must list its specific tests in the acceptance criteria: `- [ ] Test: [description] (file: [path])`
- **Backend stories** follow red-green-refactor:
  1. First criterion: write failing test(s) for the expected behavior
  2. Middle criteria: implement until tests pass
  3. Last criterion: verify all tests green
- **UI stories** include contract tests for data shapes: `- [ ] Contract test: [invoke command] returns expected shape`
- **Each story carries its own tests** — no "write all tests at the end" stories
- **Epic-level quality gates** include: `Testing trophy complete — all layers covered per the Testing Trophy section`

**Test Placement in Stories (examples):**

```markdown
### US-003: Create TaskRun entity [Backend]
**Acceptance Criteria:**
- [ ] Test: TaskRunProjection handles Started event and queries by ID (red first)
- [ ] Test: TaskRunProjection derives status from child workflow runs
- [ ] TaskRun struct with fields: task_run_id, job_id, status, ...
- [ ] AllSource projection registered
- [ ] All tests green: cargo test task_run

### US-007: Expandable accordion [UI]
**Acceptance Criteria:**
- [ ] Contract test: getTaskRunOutputs returns WorkflowRunOutputs[] with nested steps
- [ ] Contract test: approve_step_run accepts stepRunId + disposition
- [ ] Accordion expands on click, loads outputs via getTaskRunOutputs
- [ ] All tests green: bun test job-detail-runs
```

**Classify tests by deployment gate:**
- **Block merge:** Implicit/background logic (auto-completion, data filtering, orchestration) — bugs here are invisible to users
- **Block release:** Handler contracts, frontend component behavior, review action correctness
- **Follow-up OK:** E2E, visual regression, performance edge cases

**Anti-patterns to reject:**
- ❌ Shipping all stories without tests, then adding a "write tests" story at the end
- ❌ Only testing projections (base of trophy) while skipping contract and integration layers
- ❌ "It's just UI" — interactive logic (accordion, review, state machines) needs contract tests
- ❌ Tests that verify implementation details instead of behavior ("function was called" vs "output is correct")
- ❌ Acceptance criteria that say "tests added" without naming the specific tests

### 6. Functional Requirements
Numbered list of specific functionalities:
- "FR-1: The system must allow users to..."
- "FR-2: When a user clicks X, the system must..."

Be explicit and unambiguous.

### 7. Non-Goals (Out of Scope)
What this feature will NOT include. Critical for managing scope.

### 8. Technical Considerations (Optional)
- Known constraints or dependencies
- Integration points with existing systems
- Performance requirements

### 9. Success Metrics
How will success be measured?

### 10. Open Questions
Remaining questions or areas needing clarification.

---

## Writing for AI Agents

The PRD will be executed by Codex. Therefore:

- Be explicit and unambiguous
- User stories should be small (completable in one agent context window)
- Every acceptance criterion must be a `- [ ]` checkbox the agent can verify and mark `- [x]`
- Include specific file paths if you know them
- Reference existing code patterns in the project
- Tag stories with type (`[UI]`, `[Schema]`, `[Backend]`) so story-level gates are unambiguous
- Include "Mark each item [x]..." instruction in every story

---

## Output Format

**CRITICAL:** Wrap the final PRD in markers for TUI parsing:

```
[PRD]
# PRD: Feature Name

## Overview
...

## Quality Gates

### Epic-Level (run once on epic completion)
...

### Story-Level (checked per story)
...

## User Stories
...
[/PRD]
```

**File naming:** The TUI will save to `./tasks/prd-[feature-name].md`

---

## Example Conversation Flow

**User:** Create a PRD for adding dark mode to the TUI

**Assistant:** I'll help you create a PRD for dark mode. Let me ask a few questions:

1. What is the primary goal?
   A. Reduce eye strain for long sessions
   B. Match system theme preference
   C. Provide visual customization
   D. Other

2. Should the setting persist?
   A. Yes, save to config file
   B. No, session only
   C. Follow system preference

**User:** 1A, 2A

**Assistant:** Great! A couple more questions:

3. Which components need dark mode support?
   A. All TUI components
   B. Only the main execution view
   C. Main view + settings
   D. Other

4. What commands validate the whole codebase? (epic-level, run once at the end)
   A. task ci
   B. bun typecheck && bun lint
   C. make ci
   D. Other: [specify]

5. For UI stories, what story-specific checks?
   A. Verify in browser using `$codex-browser`
   B. Playwright e2e test per story
   C. Both A and B
   D. No story-level checks needed

**User:** 3A, 4B, 5A

**Assistant:** Perfect, I have enough to generate the PRD.

[PRD]
# PRD: Dark Mode for Project TUI

## Overview
Add dark mode support to the project TUI to reduce eye strain during long sessions. The feature will apply to all TUI components and persist the user's preference in the config file.

## Goals
- Provide a comfortable viewing experience in low-light environments
- Persist theme preference across sessions
- Apply consistent styling to all TUI components

## Quality Gates

### Epic-Level (run once on epic completion)
General codebase checks that run ONCE when all stories are done:
- `bun typecheck` — type checking passes with zero errors
- `bun lint` — linting passes with zero warnings

### Story-Level (checked per story)
- **UI stories:** Verify in browser using `$codex-browser`

## User Stories

### US-001: Add theme configuration [Schema]
**Description:** As a user, I want to set my preferred theme (light/dark) so that it persists across sessions.

**Acceptance Criteria:**
- [ ] Add `theme` field to the application's config schema
- [ ] Support values: "light", "dark", "system"
- [ ] Default to "light" for backwards compatibility
- [ ] Verify field is read correctly by loading config in test

Mark each item [x] as you complete it. Only close when all are checked.

### US-002: Create dark theme color palette [UI]
**Description:** As a user, I want a soft-contrast dark theme that's easy on the eyes.

**Acceptance Criteria:**
- [ ] Define dark palette with gray tones (not pure black) in theme constants file
- [ ] Ensure sufficient contrast ratios (WCAG AA) — check with contrast calculator
- [ ] Colors defined for all UI states: default, selected, hover, disabled
- [ ] Verify in browser using `$codex-browser`

Mark each item [x] as you complete it. Only close when all are checked.

### US-003: Apply theme to TUI components [UI]
**Description:** As a user, I want all TUI components to respect my theme preference.

**Acceptance Criteria:**
- [ ] Header component uses theme colors (verify in browser)
- [ ] Task list uses theme colors (verify in browser)
- [ ] Detail panels use theme colors (verify in browser)
- [ ] Progress bar uses theme colors (verify in browser)
- [ ] Dialogs use theme colors (verify in browser)
- [ ] Verify in browser using `$codex-browser`

Mark each item [x] as you complete it. Only close when all are checked.

### US-004: Add theme toggle in settings [UI]
**Description:** As a user, I want to toggle themes from within the TUI settings.

**Acceptance Criteria:**
- [ ] Theme option visible in settings view
- [ ] Changes apply immediately without restart (verify visually)
- [ ] Changes persist to the application config file
- [ ] Verify in browser using `$codex-browser`

Mark each item [x] as you complete it. Only close when all are checked.

## Functional Requirements
- FR-1: Theme setting must be readable from the application config
- FR-2: Theme must apply on TUI startup
- FR-3: Theme changes in settings must apply immediately
- FR-4: All text must maintain readability in both themes

## Non-Goals
- System theme auto-detection (future enhancement)
- Custom color schemes beyond light/dark
- Per-component theme overrides

## Technical Considerations
- Use existing OpenTUI theming capabilities if available
- Consider creating a ThemeContext for React components
- Minimize re-renders when theme changes

## Success Metrics
- All components render correctly in dark mode
- No accessibility contrast issues
- Theme persists across sessions

## Open Questions
- Should we detect system theme preference automatically in v2?
[/PRD]

---

## Checklist

Before outputting the PRD:

- [ ] Asked clarifying questions with lettered options
- [ ] Asked about quality gates with two-tier classification (REQUIRED)
- [ ] Asked about testing trophy layers (REQUIRED)
- [ ] Asked follow-up questions when needed
- [ ] Quality Gates section has Epic-Level and Story-Level subsections
- [ ] **Testing Trophy section exists** with specific tests per layer (unit, contract, integration, frontend)
- [ ] **Each backend story** names its tests in acceptance criteria (TDD: test first)
- [ ] **Each UI story** has contract test for data shape from invoke
- [ ] **Block-merge tests** identified for implicit/background logic
- [ ] Epic-level gates are general commands (typecheck, lint, ci) — NOT in individual stories
- [ ] Story-level gates are assigned to relevant stories (UI → browser, backend → endpoint check)
- [ ] Stories tagged with type: `[Schema]`, `[Backend]`, `[UI]`, `[Integration]`
- [ ] Every acceptance criterion is a verifiable `- [ ]` checkbox
- [ ] Every story includes "Mark each item [x]..." instruction
- [ ] User stories are small and independently completable
- [ ] Functional requirements are numbered and unambiguous
- [ ] Non-goals section defines clear boundaries
- [ ] PRD is wrapped in `[PRD]...[/PRD]` markers
