# PRD → Beads → Execution (chronis)

Three stages for a feature too big to hold in one context: write it down, split
it into tracked units, then work them.

There is no separate orchestrator. Codex reads the ready beads itself, and
parallelism is one worktree per agent — see
[`rules/agent-parallelism.md`](../rules/agent-parallelism.md), which also covers
when splitting is worth it and when it costs more in merge than it saves.

## Pipeline Overview

```
1. PRD Generation   →   2. Beads Creation   →   3. Execution
   (codex-prd)           (codex-beads)         (codex, reading `cn ready`)
```

## Stage 1: PRD Generation

Invoke with: "create a prd for [feature]"

The skill asks iterative clarifying questions with lettered options, then generates:

```
./tasks/prd-[feature-name].md
```

**PRD structure:**
- Overview & Goals
- Quality Gates (epic-level vs story-level)
- User Stories (tagged: `[Schema]`, `[Backend]`, `[UI]`, `[Integration]`)
- Functional Requirements
- Non-Goals
- Technical Considerations
- Success Metrics

**Key pattern:** Two-tier quality gates
- **Epic-level** — Run once at end (e.g., full E2E test suite)
- **Story-level** — Run per story (e.g., unit tests, type checks)

## Stage 2: Beads Creation

Invoke with: "create beads from [prd file]"

Converts each user story into a bead (task) using the `cn` CLI:

```bash
# Create epic
cn task create "Epic: Feature Name" --type epic --description "..." --toon

# Create child stories
cn task create "Story 1: ..." --parent <epic-id> --description "..." --toon

# Add dependencies between stories
cn dep add <story-id> <dependency-id>
```

**Output:** Beads in `.chronis/` with:
- Acceptance criteria as verifiable checkboxes
- Story-specific quality gate commands
- Dependencies between stories
- Project-detected tooling (Taskfile, Bun, etc.)

## Stage 3: Execution

```bash
codex exec 'Work ready tasks: cn ready --toon, claim each, implement, run its quality gates, then cn done.'
```

Codex then:
1. Reads the ready beads and their dependencies (`cn ready --toon` shows only
   unblocked, unclaimed work)
2. Claims one before starting it — the claim is what stops two sessions doing
   the same bead
3. Runs that story's quality gates before `cn done`
4. Runs the epic-level gates when every child bead is closed

Independent stories can go to concurrent agents, one worktree each. Split by
disjoint file sets, never by concept, and never run two builds at once —
[`rules/agent-parallelism.md`](../rules/agent-parallelism.md) has the measurements.

## Story Sizing

Each story should fit within a single agent context window. Rules:
- One concern per story
- Clear file scope (which files to create/modify)
- Self-contained acceptance criteria
- Verifiable with automated checks

## Example

```bash
# 1. Generate PRD
codex exec 'Create a PRD for user authentication with OAuth.'

# 2. Create beads from PRD
codex exec 'Create beads from ./tasks/prd-user-auth.md.'

# 3. Work them
codex exec 'Work ready tasks: cn ready --toon, claim each, implement, run its quality gates, then cn done.'
```

## CLI Reference: `br` vs `cn`

chronis (`cn`) is the successor to beads-rust (`br`) — agent-native, event-sourced,
TOON-optimized. Treat `br` as deprecated. The syntax differs:

| Command | beads-rust (`br`) | chronis (`cn`) |
|---------|-------------------|----------------|
| Create | `br create` | `cn task create --toon` |
| Dependencies | `br dep add` | `cn dep add --toon` |
| Remote sync | `br sync --flush-only` | `cn sync --toon` (configured Core only) |
| Close | `br close` | `cn done --toon` |
| Claim | `br claim` | `cn claim --toon` |
| Ready | — | `cn ready --toon` |
| Tracker flag | `--tracker beads-rust` | `--tracker chronis` |

`cn ready` has no `br` equivalent — it filters to unblocked, unclaimed work, which
is what makes the "read the ready beads yourself" execution loop possible.

## Session Protocol

```bash
# Start of session
cn ready --toon                  # Find available work
cn claim --toon <id>             # Claim a task

# During session
cn task create --toon "title" -p p1 -d "description"   # Create new tasks
cn dep add --toon <issue> <depends-on>                 # Set dependencies

# End of session
cn done --toon <id>              # Mark completed
cn list --toon                   # Verify final state

# Optional when a remote Chronis Core is configured
cn sync --toon
```

## The `--toon` flag

Pass `--toon` to **every** `cn` command. TOON output is ~50% fewer tokens than the
JSON renderer — the key differentiator from `br`, and the reason a long session can
run `cn` dozens of times without bloating context. The non-TOON renderer can also
panic on multibyte (em-dash) titles in current builds, so `--toon` is the safe
default, not just the cheap one.
