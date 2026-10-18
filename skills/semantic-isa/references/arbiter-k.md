# Research context

Primary source: Wen et al., [From Craft to Kernel: A Governance-First Execution
Architecture and Semantic ISA for Agentic Computers](https://arxiv.org/abs/2604.18652)
([HTML](https://arxiv.org/html/2604.18652v1)). Predecessor:
[ArbiterOS / From Craft to Constitution](https://arxiv.org/abs/2510.13857).

The paper proposes a model as Probabilistic Processing Unit, typed instruction
bindings and a governing execution boundary. Its vocabulary includes Cognitive,
Memory, Execution, Normative and Meta-cognitive cores, Security Context Registry,
Instruction Dependency Graph, taint propagation and recoverable violations.
These are the prototype's terms, not an established agent-computing standard.

Read the experiment and limitations sections before using benchmark figures.
State the evaluated framework, task suite, baseline, metric and remaining
failures. This reference intentionally supplies no benchmark number for reuse
without that check. A discussion of prompt-to-instruction mapping must explain
how the target implementation prevents bypasses rather than relying on the
hardware analogy.

The Codex adaptation keeps provenance and privacy labels after verification.
A check produces bounded evidence for a specific operation; it cannot prove
every external claim true or remove the need for authorisation.

Protocol references, when designing an MCP adapter:
[tools](https://modelcontextprotocol.io/specification/2026-07-28/server/tools),
[authorisation](https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization).
Verify the deployed protocol version separately from the application ISA.
