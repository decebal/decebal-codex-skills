# Exploration strategy

How to understand unfamiliar code without reading all of it. The goal is a MAP good
enough to explain the behaviour and anchor every claim to a `file:line` — not a
full read of the tree. Follow
[../../../rules/token-efficiency.md](../../../rules/token-efficiency.md): targeted
grep/glob first, `offset`/`limit` reads for the sections that matter, a full-file
read as the last resort.

## The loop

1. **Find the entry points.** These are where control enters the module:
   - `main.rs` / `main.go` / `index.ts` / `__main__.py` — process entry.
   - `mod.rs` / `lib.rs` / `index.ts` barrel / `__init__.py` — module entry and its
     public surface (`pub`, `export`, `__all__`).
   - A **route/dispatch table** — HTTP routes, a CLI command match, a message
     handler registry, an event-name switch. This is the highest-value file in a
     request-driven system: it lists every capability in one place.
   - Grep for them: `rg -n "fn main|pub fn|export (async )?function|@app.route|router\.(get|post)"`.

2. **Find the data types FIRST — before the logic.** The types name the domain and
   shrink everything else. A `struct Order`, a `type Job`, an `enum State`, a schema
   or a proto tells you what the module is ABOUT in a few lines, and the functions
   then read as transformations between known shapes.
   - `rg -n "^(pub )?(struct|enum|type|interface|class) "` scoped to the module dir.
   - Read the type definitions in full; they are small and load-bearing.

3. **Follow imports OUTWARD, one hop at a time.** From the entry point, read what it
   calls, then what THAT calls — but only along the path that answers the question.
   Do not depth-first the entire graph. Each hop, ask "does this still bear on the
   behaviour I am explaining?" and stop the branch when it does not.

4. **Grep for CALL SITES to get the other direction.** To find what depends ON the
   module (the downstream half of the dependency graph), grep the tree for the
   module's public symbols: `rg -n "OrderService|place_order" --type rust`. The
   count and locations of call sites ARE the blast radius — the reason section 5 of
   the explanation draws both directions.

5. **Read the load-bearing files, skim the rest.** A file is load-bearing if the
   behaviour changes when it changes: the entry point, the core types, the one
   function everyone calls. Config, glue, and re-exports can be skimmed or inferred
   from their names. Use `offset`/`limit` to read the relevant function, not the
   whole 2,000-line file.

## Signals that speed the map

- **A god file** (one file of many functions, or one enormous function) is where the
  behaviour concentrates — find its shape before reading top to bottom. See
  [../../../rules/layer-boundaries.md](../../../rules/layer-boundaries.md) if it
  needs opening.
- **Tests are executable documentation.** A `*_test`, `tests/`, or `*.spec` file
  shows the module's intended inputs and outputs faster than reading its
  implementation. Read a test to learn the contract, then the impl to learn how.
- **Naming conventions map layers.** `handler`/`controller`/`route` = presentation,
  `service`/`usecase` = application, `repo`/`client`/`store` = infrastructure,
  `model`/`entity`/`domain` = domain. The path often tells you the role before you
  open the file.
- **The commit that introduced it** (`git log --oneline -- <path>`, then
  `git show <hash>`) often carries the WHY that becomes section 6, and the PR body
  may link an ADR.

## When to STOP

Stop reading when the map explains the behaviour — not when every file is read.
Concretely, you are done when you can:

- state the module's purpose in one sentence,
- name its data types and its entry point with `file:line`,
- trace one request end to end through it,
- name its upstream dependencies AND its downstream consumers.

If you can do those four, write the explanation. Reading further spends tokens
without changing the answer. If a branch remains genuinely unexplored, name it as a
gap in the explanation rather than reading it "to be safe" — the reader can ask you
to go deeper on exactly that branch.

## Verify before you assert

Every `file:line` in the output must point at what you claim it does, and every
arrow in the diagrams must reflect a dependency you actually saw — not one you
inferred from a name. A confident wrong anchor is worse than an admitted gap: see
[../../../rules/evidence-discipline.md](../../../rules/evidence-discipline.md).
