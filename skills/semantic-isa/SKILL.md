---
name: semantic-isa
description: Design a typed Semantic Instruction Set Architecture for an agent or MCP workflow, with deterministic admission, provenance, scoped verification and refusal tests; or explain Arbiter-K accurately. Use for semantic ISA, agent execution kernels, tool-call taint tracking and governance contracts, rather than ordinary API design or a general security audit.
---

# Semantic ISA

Separate model proposals from executable authority. Translate a named agent's
capabilities into a closed instruction set, then specify what a deterministic
admission function permits from a captured policy and evidence snapshot.
This skill designs that contract; loading it installs no runtime enforcement,
grants no tools and authorises no external action.

Choose **DESIGN** for a system contract or **WRITE** for an explanation. Read
[research context](references/arbiter-k.md) before attributing claims or numbers
to the paper. Use [the specification template](references/isa-template.md) for
DESIGN. Read [Codex tool integration](references/codex-tools.md) when the system
uses this Mac's connectors, memory, browser, MCP or recurring work.

## DESIGN

1. Read the repository's instructions, architecture and relevant runtime
   contracts. Inventory capabilities from actual tools, source and authorised
   configuration. Distinguish available, configured, granted, proposed and
   demonstrated. Ask only for consequential missing decisions; do not ask the
   user to recite tools that can be inspected.
2. List side effects and data disclosure: reads can expose private content;
   simulations can incur model cost; browser navigation can send data. Include
   storage, network, messages, paid queries, delegation and scheduled work.
   Name what is outside the requested agent and cannot be reached by its kernel.
3. Map capabilities to domain-specific instructions. Avoid an unrestricted
   `tool_call(name, args)` escape hatch. Classify instructions using the five
   cores below, or document an explicit domain adaptation. Every operation needs
   a governance property, binding, authority requirement and error contract.

   | Core | Role | Boundary |
   | --- | --- | --- |
   | Cognitive | Generate, decompose, reflect | Untrusted proposals, never execution authority |
   | Memory | Load, store, transform, render | Preserve lineage; lossy/model transforms are not deterministic facts |
   | Execution | Domain writes, external tools, delegation, responses | Check each effect and disclosure destination |
   | Normative | Verify, constrain, refuse, interrupt | Trusted checks; models may request but cannot issue their own proof |
   | Meta-cognitive | Progress estimates, routing, resource observation | Advice cannot increase grants or budgets |

4. Define closed input/output schemas: units, enums, bounds, unknowns, resource
   identities, revisions and version negotiation. Reject malformed values;
   any legitimate normalization must be explicit, deterministic and auditable.
   Keep retrieved instructions as data. Typed syntax alone proves no factual
   truth, user intent or right to act.
5. Build the Security Context Registry and Instruction Dependency Graph.
   Track provenance, confidentiality, integrity/review state and authority as
   separate dimensions. Trace tags through storage, caching, rendering and
   delegation. For each sink, name its required checks and accepted evidence.
6. Specify `VERIFY` as a named deterministic predicate over captured inputs,
   policy, principal, resource, destination, revision and expiry. Its receipt
   discharges only that predicate for those exact inputs. Preserve original
   provenance and privacy labels. Never implement a universal `trusted = true`
   after a schema check or model review. External facts may remain unknown.
7. Define structured refusals and recovery. Include stale revisions, revoked
   authority, conflicting idempotency keys, uncertain remote writes and resource
   limits. Reconciliation precedes retry when a side effect may already exist.
   No silent coercion, invented success or automatic extra spend.
8. Draw container and sequence diagrams. Trace one refused sink, a bounded
   verification, then the permitted or still-refused outcome. Distinguish a
   reproducible admission decision from nondeterministic external execution.
9. Plan deterministic fixtures: one refusal/success pair per sink, failed
   verification, provenance round trip, replay, concurrency and recovery.
   Drive scripted proposals, not a live model. Test the real write boundary;
   a diagram or a green schema validator is not enforcement proof.

Keep the pure admission predicate and domain types free of I/O, ambient clocks
and model calls. The application gathers current authenticated state, captures
an explicit as-of time, runs admission and rechecks volatile authority at commit.
Infrastructure executes effects and persists receipts. TOCTOU protection,
durable reservations and destination readback belong in that execution contract.

Compose other skills only when useful and installed: `brainstorming` for product
scope, `arch` for diagrams, `rust-clean-architecture` for placement,
`security-review` for sink analysis, and `integration-test` for executable proof.
If implementation planning is requested, use `codex-prd` and `codex-beads` with
the actual tracker (`cn --toon` on this Mac). Missing optional skills are not a
reason to invent tools or stop a specification that can be completed directly.

## WRITE

Describe Arbiter-K as a 2026 research proposal/prototype. Its hardware analogy
specifies an execution boundary; it does not make an LLM or remote service
deterministic. Distinguish the paper's vocabulary from your own application ISA.
Read original sources before quoting figures; include benchmark conditions,
residual failures and limitations. If a source cannot be checked, omit its
numbers or mark them unverified. Do not turn a secondary critique into a fact
about the implementation. Follow the user's writing/voice guidance when present.

## Completion and rollout

Every in-scope capability maps to an instruction and every sink has its schema,
policy, proof requirements, refusal and recovery tests. List uncovered surfaces
and unresolved authority rather than claiming the whole environment is governed.
Save the design in the target repository's indexed `docs/` layout. Commit or
publish only with the user's existing authorisation and repository rules.

A proposed kernel can run shadow evaluation on captured fixtures or beside
existing controls. Shadow mode never bypasses those controls or enables effects
that current policy refuses. Enforce only after reviewing refusal traces and
proving the actual executor cannot route around admission. Documentation and
skill installation alone do not meet that bar.
