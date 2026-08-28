---
name: explain-module
description: Explore an unfamiliar module/component/system and explain how it works, backed by real file:line anchors and Mermaid diagrams. Use when someone asks "how does X work?", "explain the Y module", "what does Z do?", or "walk me through ...". Triggers on any request to understand, orient in, or onboard onto a piece of a codebase.
---

# Explain a module

Turn "how does X work?" into a structured, evidence-backed explanation. The output
is always the same shape, so a reader knows where to look for each answer.

## Read the code FIRST — efficiently

Never explain from the name or from memory. Map the code, then read only the
load-bearing files. Follow [../../rules/token-efficiency.md](../../rules/token-efficiency.md):
grep/glob to build the map, `offset`/`limit` reads for the sections that matter, a
full-file read as the last resort. See
[references/exploration-strategy.md](references/exploration-strategy.md) for the
entry-points → types → call-sites loop and when to stop.

Anchor every claim to code you actually read. If you did not read it, do not assert it.

## Two modes — pick before writing

| Mode | When | Sections |
|---|---|---|
| **Quick** | orientation, "just give me the gist" | Purpose + Key Files + ONE diagram (data flow) |
| **Full** | onboarding, a change is coming, a design review | all 7 sections below |

Default to Quick unless the ask is clearly deep. Offer to expand.

## Output structure (Full mode)

1. **Purpose** — 1-2 sentences. The problem this module solves, not a restatement
   of its name.
2. **Architecture Context** — where it sits in the system and who calls it. If a
   C4 view exists, reference it; if one would help and none exists, offer to make
   one with the sibling [../arch/SKILL.md](../arch/SKILL.md) skill.
3. **Key Files** — a bullet per file, each `path:line — one-line role`. Use REAL
   `file:line` anchors from your reads so they are clickable. 3-8 bullets; if you
   need more, the module is really several — say so.
4. **Data Flow** — a Mermaid `sequenceDiagram` of how data moves through it for the
   main scenario. Template in
   [references/diagram-patterns.md](references/diagram-patterns.md).
5. **Dependency Graph** — a Mermaid `flowchart` showing BOTH directions: what the
   module depends on (upstream) and what depends on it (downstream). Template in
   the same reference.
6. **Key Design Decisions** — why it is built this way, not just what it is. Link
   any ADR / decision record you find (`docs/adr/`, `DECISIONS.md`, RFCs). Note
   trade-offs and non-obvious constraints.
7. **Common Modifications** — the 2-4 changes a dev typically makes here and the
   exact file:line each one starts from ("add a new X → register in `foo.rs:120`").

Quick mode emits sections 1, 3, and the section-4 sequence diagram only.

## Steps

1. **Locate** the module — glob for the path, grep the name across the tree to find
   its call sites and its own entry point.
2. **Map** it — list its files, find the data types first (they name the domain),
   then the entry point (`main`, `index`, `mod.rs`, a route table, a public
   `pub fn`/exported symbol).
3. **Read** the load-bearing files only, following imports outward one hop at a time.
   Stop when the map explains the behaviour — not when every file is read.
4. **Write** the sections for the chosen mode, filling both Mermaid diagrams with
   REAL names (never `ModuleA`, `foo`) and captioning each with its legend.
5. **Verify** every `file:line` anchor points at what you claim and every arrow in
   the diagrams reflects a dependency you actually saw in the code.

## Rules

- **Anchors are real or absent.** A wrong `file:line` is worse than none — it sends
  the reader to the wrong place and erodes trust in the whole explanation.
- **Diagrams use real component names**, and every arrow is labelled with what
  crosses it (a call, a protocol, an event). An unlabelled arrow is an unanswered
  question.
- **Explain the current code**, not its history. No "this used to…", no narration
  of your exploration. State what is true now.
- **Say what you did not read.** If a branch of the module went unexplored, name it
  as a gap rather than guessing what it does.
