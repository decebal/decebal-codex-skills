---
name: arch
description: Generate architecture diagrams as MermaidJS using the C4 model — System Context (L1), Container (L2), Component (L3), Code (L4, usually skipped), plus a data-flow sequence diagram. Use when the user asks to "draw architecture", "create a diagram", "document system structure", "diagram the system", "show how the components fit together", or wants a C4 / architecture / data-flow diagram embedded in Markdown.
---

# Architecture Diagrams (C4 + Mermaid)

Produce architecture diagrams as MermaidJS fenced code blocks embedded in a `.md`
file, using the C4 model. C4 has four nested zoom levels; pick the ONE the reader
needs and draw exactly that level.

| Level | Name | Answers | Audience | Mermaid type |
|---|---|---|---|---|
| L1 | System Context | How does the system fit into the world? Who uses it, what does it talk to? | everyone | `C4Context` (or `flowchart`) |
| L2 | Container | What deployable/runnable pieces make it up (apps, services, DBs)? | developers, ops | `C4Container` (or `graph LR`) |
| L3 | Component | What are the major parts inside ONE container? | developers of that container | `C4Component` (or `flowchart`) |
| L4 | Code | Classes/functions inside one component | rarely useful — **usually skip** | `classDiagram` |
| — | Data flow | What is the ordered sequence of messages for one scenario? | developers | `sequenceDiagram` |

L4 is almost always noise the IDE already shows — default to skipping it. Most
useful deliverables are L1 + L2, plus a data-flow diagram for the tricky path.

## Steps

1. **Identify the level the reader needs.** Ask "who reads this and what decision
   do they make?" A newcomer orienting → L1. A developer wiring services → L2. A
   developer inside one service → L3. Debugging an ordered exchange → data flow.
   When unsure, start at L1 and only go deeper where a real question remains.
2. **Pick the Mermaid diagram type** from the table above. Prefer the plain
   `flowchart`/`graph`/`sequenceDiagram` types unless you specifically want C4's
   styled shapes — the `C4*` types are experimental (see caveat below).
3. **Fill the matching template** from
   [references/c4-templates.md](references/c4-templates.md). Replace every
   placeholder with a real component name and a real technology/relationship. Do
   not ship a template with `System A` / `foo` still in it.
4. **Embed it in a `.md`** inside a ```` ```mermaid ```` fenced block, under a
   heading naming the level, with a one-line caption that carries the legend
   (see rules). GitHub, GitLab and most doc tools render these blocks inline.

## Rules

- **One level per diagram.** Never mix a context view and a component view in the
  same diagram — that is the single fastest way to make a C4 diagram unreadable.
  Draw separate diagrams and stack them in the `.md`, zooming in as you go down.
- **Name every box with a real component**, never an opaque id. `Payments Service`,
  `Postgres (orders)`, `Stripe API` — not `svc-1`, `node_a`, `cq-01kr…`. The label
  the reader sees must be a name they can act on; a diagram of anonymous boxes
  documents nothing. (Mermaid node ids like `web` are fine — they are invisible;
  the *display text* is what must be a real name.)
- **Keep context (L1) diagrams to under ~10 boxes.** If it needs more, you are
  drawing a container diagram — drop to L2. A context diagram is the system, its
  users, and the external systems it talks to; nothing else.
- **Put the legend in the caption**, not in floating boxes on the canvas. One
  italic line under the diagram: what the shapes/colors/arrows mean and the
  scenario, e.g. *"C4 Container view. Boxes = deployable units, cylinders =
  datastores, arrows = synchronous HTTP unless labelled."*
- **Label every arrow** with the relationship or protocol (`reads from`, `HTTPS`,
  `publishes to`) — an unlabelled arrow is an unanswered question.
- Diagrams should reflect the real layering. If the system follows the
  presentation → application → infrastructure ← domain direction, the diagram
  should show it (and dependency arrows point the legal way). See
  [../../rules/layer-boundaries.md](../../rules/layer-boundaries.md).

## Mermaid C4 caveat

Mermaid's dedicated C4 diagram types (`C4Context`, `C4Container`, `C4Component`)
are **experimental** — the syntax and especially auto-layout are unstable and can
render poorly or differ across Mermaid versions and renderers. When fidelity or
portability matters, use a plain `flowchart`/`graph` (for the structural levels)
or `sequenceDiagram` (for data flow) instead: they are stable, lay out
predictably, and every renderer supports them. `references/c4-templates.md`
provides both a `C4*` template and a plain-diagram fallback for each level.
