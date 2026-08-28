---
name: codex-beads
description: "Convert a PRD to beads (epic plus child tasks) for Codex execution. Auto-detect the installed tracker CLI—Chronis (cn), beads-rust (br), or beads (bd)—and create one epic with a child task per user story. Use when asked to create beads, convert a PRD to beads, or prepare Chronis work for Codex."
---

# Codex — Create Beads

Converts a PRD to beads (epic + child tasks) for Codex execution. Pairs
with the [`codex-prd`](../codex-prd/SKILL.md) skill, which generates the PRD.

The examples below use **chronis** (`cn`) — the primary tracker. This skill
auto-detects which tracker CLI is installed (see *Detect the Tracker CLI* below); if
it's `br` or `bd` rather than `cn`, substitute its syntax using the
[differences table](#differences-across-beads-clis) at the bottom.

---

## The Job

Take a PRD (markdown file or text) and create beads using `cn` commands:
1. **Detect project tooling** (build runner, package manager)
2. **Classify Quality Gates** into story-specific vs. epic-level
3. Create an **epic** bead (with epic-level quality gates)
4. Create **child beads** for each user story (with story-specific acceptance criteria only)
5. Set up **dependencies** between beads (schema → backend → UI)
6. Output a ready-to-run `codex exec` command for the execution loop.

---

## Detect the Tracker CLI

Do this before anything else — it decides which command syntax the rest of this
skill uses. Pick the first that applies:

1. `cn` on `PATH` (or a `.chronis/` dir in the repo) → **chronis**, the primary
   tracker. Use `cn` with `--toon` on every command, exactly as written below.
2. else `br` on `PATH` → **beads-rust**. Use `br` syntax (no `--toon`).
3. else `bd` on `PATH` (or a `.beads/` dir) → **beads** (Go). Use `bd` syntax.

```bash
if command -v cn >/dev/null 2>&1; then TRACKER=cn
elif command -v br >/dev/null 2>&1; then TRACKER=br
elif command -v bd >/dev/null 2>&1; then TRACKER=bd
else echo "no bead tracker found (cn/br/bd)"; fi
```

The body below is written for `cn`. For `br`/`bd`, translate each command with the
[differences table](#differences-across-beads-clis) — the shapes are identical
(create → dep add → close), only the binary name and a few flags change.

---

## Step 0: Detect Project Tooling

Before creating beads, detect the project's build runner and package manager. This determines which commands appear in quality gates.

### Build runner detection

Check in order:
1. `Taskfile.yml` or `Taskfile.yaml` exists → use `task` (go-task)
2. `Makefile` exists → use `make`
3. Neither → use raw commands

```bash
# Detect build runner
if [ -f Taskfile.yml ] || [ -f Taskfile.yaml ]; then
  RUNNER="task"    # e.g., task ci, task lint
elif [ -f Makefile ]; then
  RUNNER="make"    # e.g., make ci, make lint
else
  RUNNER="none"    # use raw commands directly
fi
```

When detected, scan the file for available targets to use in quality gates:
- `task`: read `Taskfile.yml` for task names (e.g., `task ci`, `task lint`, `task test`)
- `make`: read `Makefile` for target names (e.g., `make ci`, `make lint`, `make test`)

### Package manager

Always use **bun** as the JS/TS package manager:
- `bun install` (not npm/pnpm install)
- `bun run <script>` (not npm/pnpm run)
- `bun test` (not npm/pnpm test)
- `bunx <package>` (not npx)
- `bun typecheck` or `bun run typecheck` for type checking

### E2E testing with Playwright

If the project has Playwright configured (check for `playwright.config.ts` or `@playwright/test` in package.json), use it for:
- **E2E tests**: `bunx playwright test` — runs all end-to-end tests
- **Visual regression**: `bunx playwright test --update-snapshots` to update baselines, `bunx playwright test` to compare
- **Single test file**: `bunx playwright test tests/my-feature.spec.ts`
- **Show report**: `bunx playwright show-report`

Playwright gates classify as:
- **Story-level**: `bunx playwright test tests/<this-story>.spec.ts` — when a story adds/modifies a specific e2e test
- **Epic-level**: `bunx playwright test` (full suite) — run on epic completion to catch regressions across stories

For UI stories that need visual diff verification, the acceptance criteria should include:
- `- [ ] Playwright e2e test passes: bunx playwright test tests/<feature>.spec.ts`
- `- [ ] Visual snapshot updated and matches expected (bunx playwright test --update-snapshots)`

### Detect the project runtime first

This skill applies to any chronis project, not just JS/TS. Before running any
gate, detect the runtime from what's in the repo and use ITS runner — the bun
table below is the JS/TS case, not a universal default:

- `Cargo.toml` → `cargo test` / `cargo clippy -- -D warnings` / `cargo fmt --check`
- `mix.exs` → `mix test` / `mix format --check-formatted` / `mix credo`
- `go.mod` → `go test ./...` / `go vet ./...` / `gofmt -l`
- `bun.lock` / `package.json` → the bun table below
- A `Taskfile.yml` / `Makefile` with the right target (`task ci`, `make test`) wins over all of the above — prefer it

### Quality gate command mapping (JS/TS projects)

| What | Command |
|------|---------|
| Type checking | `bun typecheck` or `bun run typecheck` |
| Linting | `bun lint` or `bun run lint` |
| Unit tests | `bun test` |
| E2E tests | `bunx playwright test` |
| E2E single file | `bunx playwright test tests/<file>.spec.ts` |
| Visual snapshots | `bunx playwright test --update-snapshots` |
| Full CI | `task ci` / `make ci` / `bun run ci` |
| Format check | `bun run format:check` |
| Exec a package | `bunx <pkg>` |

---

## Quality Gates: Two Tiers

Quality gates split into two tiers based on scope:

### Story-Level Gates (checked per bead)

Criteria that verify THIS story's specific deliverable. The agent MUST check each item individually and mark `- [x]` before closing the bead.

Examples:
- `- [ ] Add investorType column with default 'cold'`
- `- [ ] Toggle shows confirmation dialog`
- `- [ ] Filter persists in URL params`
- `- [ ] Verify in browser using $codex-browser` (UI stories only)
- `- [ ] Playwright e2e test passes: bunx playwright test tests/toggle.spec.ts` (when story has a specific test)
- `- [ ] Visual snapshot updated and matches expected` (when story changes visible UI)

### Epic-Level Gates (checked on epic completion only)

General/universal commands that validate the whole codebase. These go in the **epic description**, NOT in individual stories. They run once when all stories are done.

Examples:
- `bun typecheck`
- `bun lint`
- `task ci` / `make ci`
- `bun test`
- `bunx playwright test` (full e2e suite)
- `cargo test`
- `mix test`

**Why?** Running `bun typecheck` after every single story wastes agent context and time. Intermediate stories may legitimately have temporary type errors that the next story fixes. The full Playwright suite also catches cross-story regressions better when run once at the end.

### Extracting Gates from a PRD

Look for the "Quality Gates" section in the PRD. Translate any `pnpm`/`npm`/`npx` references to `bun`/`bunx`:

```markdown
## Quality Gates
- `bun typecheck` - Type checking
- `bun lint` - Linting

For UI stories:
- Verify in browser using `$codex-browser`
```

Classify each gate:
- `bun typecheck` → **epic-level** (general codebase check)
- `bun lint` → **epic-level** (general codebase check)
- `Verify in browser` → **story-level** (specific to UI stories, add to those stories)

If the project has a `task ci` or `make ci` target, prefer that as the single epic-level gate since it typically runs all checks.

**If no Quality Gates section exists:** Ask the user what commands should pass and how to classify them.

---

## Acceptance Criteria: Check Each Item

**CRITICAL:** The agent working on a bead MUST individually verify and mark each acceptance criteria item as done. This is not optional.

### How it works

Each bead's description contains `- [ ]` checkboxes. The agent must:

1. Work through each criterion
2. Verify it is satisfied (run a command, check output, inspect code)
3. Run `cn show <id> --toon`, preserve the full current description, and change only
   that criterion from `- [ ]` to `- [x]`
4. Replace the description by piping the updated full text to
   `cn task edit <id> --description - --toon`
5. Only close the bead when ALL items are `- [x]`

### Writing verifiable criteria

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

---

## Output Format

Beads use `cn task create` with **HEREDOC syntax** to safely handle special characters:

```bash
# Create epic with epic-level quality gates
cn task create "[Feature Name]" --toon --type epic \
  --description "$(cat <<'EOF'
[Feature description from PRD]

## Epic Quality Gates (run on completion)
- [ ] `task ci` passes (or `make ci`, or individual commands)
- [ ] `bun typecheck` passes
- [ ] `bun lint` passes
EOF
)"

# Create child bead with story-specific criteria only
cn task create "[Story Title]" --toon \
  --parent EPIC_ID \
  --description "$(cat <<'EOF'
[Story description]

## Acceptance Criteria
- [ ] Specific verifiable criterion 1
- [ ] Specific verifiable criterion 2
- [ ] Specific verifiable criterion 3

Mark each item [x] as you complete it. Only close when all are checked.
EOF
)" \
  --priority p1
```

> **CRITICAL:** Always use `<<'EOF'` (single-quoted) for the HEREDOC delimiter. This prevents shell interpretation of backticks, `$variables`, and `()` in descriptions.

---

## Story Size: The #1 Rule

**Each story must be completable in one Codex context window.**

A task may be resumed in a fresh context. Keep each task self-contained so its
description carries every requirement needed to finish it.

### Right-sized stories:
- Add a database column + migration
- Add a UI component to an existing page
- Update a server action with new logic
- Add a filter dropdown to a list

### Too big (split these):
- "Build the entire dashboard" → Split into: schema, queries, UI components, filters
- "Add authentication" → Split into: schema, middleware, login UI, session handling
- "Refactor the API" → Split into one story per endpoint or pattern

**Rule of thumb:** If you can't describe the change in 2-3 sentences, it's too big.

---

## Story Ordering: Dependencies First

Stories execute in dependency order. Earlier stories must not depend on later ones.

**Correct order:**
1. Schema/database changes (migrations)
2. Server actions / backend logic
3. UI components that use the backend
4. Dashboard/summary views that aggregate data

---

## Dependencies with `cn dep add`

```bash
cn dep add --toon feature-002 feature-001  # US-002 depends on US-001
cn dep add --toon feature-003 feature-002  # US-003 depends on US-002
```

**Syntax:** `cn dep add <issue> <depends-on>` — the issue depends on (is blocked by) depends-on.

---

## Conversion Rules

1. **Detect tooling**: prefer a Taskfile.yml/Makefile target; else use the project runtime's runner (see "Detect the project runtime first") — bun/bunx for JS/TS, cargo for Rust, mix for Elixir, go for Go
2. **Classify Quality Gates** from PRD into story-level vs. epic-level
3. **Epic-level gates** go in the epic description (`task ci`/`make ci`, `bun typecheck`, `bun lint`)
4. **Story-level gates** go in individual bead descriptions (browser verification for UI stories)
5. **Each user story → one bead** with only story-specific acceptance criteria
6. **Every criterion** must be a `- [ ]` checkbox the agent can verify and mark `- [x]`
7. **First story**: No dependencies (creates foundation)
8. **Subsequent stories**: Depend on their predecessors
9. **Priority**: Based on dependency order, then document order (`p0` highest through `p3` lowest)
10. **All stories**: Include the instruction to mark items `[x]` and only close when all checked

---

## Example

**Input PRD:**
```markdown
# PRD: Friends Outreach

Add ability to mark investors as "friends" for warm outreach.

## Quality Gates
- `bun typecheck` - Type checking
- `bun lint` - Linting

For UI stories:
- Verify in browser using `$codex-browser`

## User Stories

### US-001: Add investorType field to investor table
- [ ] Add investorType column: 'cold' | 'friend' (default 'cold')
- [ ] Generate and run migration successfully

### US-002: Add type toggle to investor list rows
- [ ] Each row has Cold | Friend toggle
- [ ] Switching shows confirmation dialog
- [ ] On confirm: updates type in database

### US-003: Filter investors by type
- [ ] Filter dropdown: All | Cold | Friend
- [ ] Filter persists in URL params
```

**Tooling detected:** `Taskfile.yml` exists → use `task`. Has `task ci` target.

**Gate classification:**
- `bun typecheck` → epic-level (covered by `task ci`)
- `bun lint` → epic-level (covered by `task ci`)
- `Verify in browser` → story-level (UI stories only)

**Output beads:**
```bash
# Create epic with epic-level quality gates
cn task create "Friends Outreach Track" --toon --type epic \
  --description "$(cat <<'EOF'
Warm outreach for deck feedback.

## Epic Quality Gates (run on completion)
- [ ] `task ci` passes (includes typecheck + lint)
EOF
)"

# US-001: Schema story (no browser gate, no deps)
cn task create "US-001: Add investorType field to investor table" --toon \
  --parent feature-abc \
  --description "$(cat <<'EOF'
As a developer, I need to categorize investors as 'cold' or 'friend'.

## Acceptance Criteria
- [ ] Add investorType column: 'cold' | 'friend' (default 'cold')
- [ ] Generate and run migration successfully
- [ ] Verify column exists: check schema file or run migration dry-run

Mark each item [x] as you complete it. Only close when all are checked.
EOF
)" \
  --priority p0

# US-002: UI story (includes browser verification gate)
cn task create "US-002: Add type toggle to investor list rows" --toon \
  --parent feature-abc \
  --description "$(cat <<'EOF'
As Ryan, I want to toggle investor type directly from the list.

## Acceptance Criteria
- [ ] Each row has Cold | Friend toggle
- [ ] Switching shows confirmation dialog
- [ ] On confirm: updates type in database
- [ ] Verify in browser using `$codex-browser`

Mark each item [x] as you complete it. Only close when all are checked.
EOF
)" \
  --priority p1

cn dep add --toon feature-002 feature-001

# US-003: UI story (includes browser verification gate)
cn task create "US-003: Filter investors by type" --toon \
  --parent feature-abc \
  --description "$(cat <<'EOF'
As Ryan, I want to filter the list to see just friends or cold.

## Acceptance Criteria
- [ ] Filter dropdown: All | Cold | Friend
- [ ] Filter persists in URL params
- [ ] Verify in browser using `$codex-browser`

Mark each item [x] as you complete it. Only close when all are checked.
EOF
)" \
  --priority p2

cn dep add --toon feature-003 feature-002
```

---

## Output Location

Beads are stored in: `.chronis/` (WAL + Parquet event files — there is no separate task database)

After creation, run Codex:
```bash
codex exec 'Work ready tasks under epic feature-abc. Use cn ready --toon, claim each task, implement it, run its quality gates, then cn done.'
```

Codex will:
1. Pick and claim an unblocked task
2. Agent works through each `- [ ]` item, marking `- [x]` as verified
3. Agent closes the bead only when all criteria are `- [x]`
4. When all child beads are closed, Codex runs the epic-level quality gates
5. If epic gates pass, closes the epic and outputs `<promise>COMPLETE</promise>`
6. If epic gates fail, creates a fix-up bead to address the failures

---

## Checklist Before Creating Beads

- [ ] Detected build runner (Taskfile.yml → `task`, Makefile → `make`, or raw commands)
- [ ] Using `bun`/`bunx` for all JS/TS commands (not pnpm/npm/npx)
- [ ] Quality gates classified: story-level vs. epic-level
- [ ] Epic description includes epic-level gates as `- [ ]` checkboxes
- [ ] Each story has ONLY story-specific criteria (no typecheck/lint)
- [ ] Every criterion is a verifiable `- [ ]` checkbox
- [ ] UI stories include browser verification (if in PRD quality gates)
- [ ] Each story includes "Mark each item [x]..." instruction
- [ ] Stories are right-sized (one agent context window)
- [ ] Dependencies set with `cn dep add`

---

## CLI Command Reference

| Command | Purpose |
|---------|---------|
| `cn task create "title" --toon` | Create a new task |
| `cn task create "title" --toon --type epic` | Create an epic |
| `cn task create "title" --toon --parent ID` | Create a child task |
| `cn dep add --toon <issue> <blocked-by>` | Add a dependency |
| `cn claim <id> --toon` | Claim ready work |
| `cn done <id> --toon --reason "..."` | Complete a task |
| `cn task edit <id> --toon` | Edit a task |

---

## Differences across beads CLIs

Step 0 picks one of these. Translate the `cn` commands above to the detected CLI:

| Command | beads (`bd`) | beads-rust (`br`) | chronis (`cn`) |
|---------|--------------|-------------------|----------------|
| Create | `bd create` | `br create` | `cn task create "title" --toon` |
| Dependencies | `bd dep add` | `br dep add` | `cn dep add --toon` |
| Close | `bd close` | `br close` | `cn done --toon` |
| Storage | `.beads/beads.jsonl` | `.beads/*.db` + JSONL | `.chronis/` (WAL + Parquet) |
