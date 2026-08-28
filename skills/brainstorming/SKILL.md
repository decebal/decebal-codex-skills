---
name: brainstorming
description: "You MUST use this before any creative work - creating features, building components, adding functionality, or modifying behavior. Explores user intent, requirements and design before implementation."
---

# Brainstorming Ideas Into Designs

## Overview

Help turn ideas into fully formed designs and specs through natural collaborative dialogue.

Start by understanding the current project context, then ask questions one at a time to refine the idea. Once you understand what you're building, present the design in small sections (200-300 words), checking after each section whether it looks right so far.

## The Process

**Understanding the idea:**
- Check out the current project state first (files, docs, recent commits)
- Ask questions one at a time to refine the idea
- Prefer multiple choice questions when possible, but open-ended is fine too
- Only one question per message - if a topic needs more exploration, break it into multiple questions
- Focus on understanding: purpose, constraints, success criteria

**Exploring approaches:**
- Propose 2-3 different approaches with trade-offs
- Present options conversationally with your recommendation and reasoning
- Lead with your recommended option and explain why

**Presenting the design:**
- Once you believe you understand what you're building, present the design
- Break it into sections of 200-300 words
- Ask after each section whether it looks right so far
- Cover: architecture, components, data flow, error handling, testing
- Be ready to go back and clarify if something doesn't make sense

## After the Design

**Documentation:**
- Write the validated design to `docs/plans/YYYY-MM-DD-<topic>-design.md`
- Use elements-of-style:writing-clearly-and-concisely skill if available
- Commit the design document to git
- Register in the per-repo `.prime/` graph (see "Prime Indexing" below)

**Prime Indexing:**
After writing the design doc, register it as a node so `create-plans` and future brainstorming sessions can recall it:

1. **Before starting the dialogue** (at the very beginning of the brainstorm, when the user's initial idea is known), call `mcp__prime__prime_recall { text: <user's stated idea>, top_k: 5, depth: 1 }` without `node_type` filter — surface ANY related prior work (brainstorms, plans, prompts) in this codebase. If matches above ~0.75 exist, mention them inline before asking the first question:

"Related prior work in this repo: docs/plans/2026-04-12-vault-design.md (similarity 0.82). Want me to read it first?"

2. `mcp__prime__prime_add_node`:
   - type: `"brainstorm"`
   - properties: `{ name: "<topic>-design", file: "docs/plans/YYYY-MM-DD-<topic>-design.md", topic: "<kebab-topic>", domain: <inferred>, created_at: <YYYY-MM-DD> }`
   - Capture entity_id.

3. `mcp__prime__prime_embed { id: <entity_id>, text: <validated design's one-liner + core architectural decisions, ~2–3 sentences> }` — server embeds via in-process fastembed. Enables future brainstorming and planning invocations to discover this design.

4. For each source path listed in the design body, ensure a `file` node exists (search by `path`, create if missing) and `mcp__prime__prime_add_edge { source: <brainstorm>, target: <file>, relation: "references" }`.

5. If this brainstorm refined or replaced an earlier design on the same topic, also `mcp__prime__prime_add_edge { source: <new>, target: <prior>, relation: "supersedes" }`.

Best-effort: Prime failures don't block git commit or implementation handoff. Requires `allsource-prime ≥ 0.21.3`.

**Implementation (if continuing):**
- Ask: "Ready to set up for implementation?"
- Use an isolated git worktree when parallel implementation warrants one
- Use superpowers:writing-plans to create detailed implementation plan

## Key Principles

- **One question at a time** - Don't overwhelm with multiple questions
- **Multiple choice preferred** - Easier to answer than open-ended when possible
- **YAGNI ruthlessly** - Remove unnecessary features from all designs
- **Explore alternatives** - Always propose 2-3 approaches before settling
- **Incremental validation** - Present design in sections, validate each
- **Be flexible** - Go back and clarify when something doesn't make sense
