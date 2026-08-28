# Creating Custom Skills

## Skill Structure

A skill is a directory with at minimum a `SKILL.md` file:

```
my-skill/
├── SKILL.md           # Required: instructions and knowledge
├── references/        # Optional: additional context files
│   ├── guide.md
│   └── examples.md
└── package.json       # Optional: if skill needs dependencies
```

## SKILL.md Format

```markdown
---
name: my-skill
description: "What this skill does and the user requests or contexts that should activate it."
---

# Skill Title

## When to Use
{Describe when this skill activates}

## Instructions
{Step-by-step behavior for Codex}

## Rules
{Constraints and guidelines}
```

## Key Principles

1. **Specific discovery text** — Put clear trigger phrases in `description`
2. **Actionable instructions** — Tell Codex exactly what to do, not just what to know
3. **Reference files for depth** — Keep SKILL.md concise; put detailed guides in `references/`
4. **Verifiable outputs** — Include acceptance criteria or checkboxes for validation
5. **Tool awareness** — Name required capabilities without inventing provider-specific tool aliases

## Composing Skills

Skills chain: a PRD feeds a beads converter, while a review may also use a
language skill. Express relationships in skill instructions and default prompts;
Codex skill frontmatter stays limited to `name` and `description`. See
[skill-composition.md](skill-composition.md).

## Examples from Installed Skills

### Questionnaire Pattern (codex-prd)
Ask lettered clarifying questions (A, B, C, D) before generating output. Adapt follow-up questions based on answers.

### Multi-phase Workflow (mcp-builder)
1. Research & Planning
2. Implementation
3. Review & Test
4. Evaluation with scoring

### Rule-based Knowledge (feature-spec, typescript)
Organize knowledge as numbered rules with category prefixes (e.g., `scope-01`, `req-03`). Allows precise referencing.

### Auto-trigger (brainstorming)
Mark as "MUST use before any creative work" to auto-activate without explicit invocation.

## Installation

```bash
# Symlink into Codex's skills directory
ln -s /path/to/my-skill ~/.agents/skills/my-skill

# Or install for one repository
mkdir -p .agents/skills
cp -R /path/to/my-skill .agents/skills/my-skill
```

## Testing

1. Start a new Codex session
2. Invoke `$my-skill` and test representative implicit trigger phrases
3. Verify the skill activates and follows its instructions
4. Check that outputs match expected format
