# Evaluating an MCP Server with Codex

Evaluate whether a model can solve realistic tasks through server tools, not
whether each tool returns a canned fixture.

## Evaluation shape

Each case needs:

- stable setup and fixture data;
- one natural-language user request;
- expected answer or invariant;
- expected tool family, when tool selection matters;
- explicit read-only or mutation classification;
- deterministic cleanup for mutation cases.

Prefer historical or fixture-backed questions. Live totals, current rankings,
and relative dates produce false failures when source data changes.

## Good cases

Test workflows humans ask for:

- search, filter, and aggregate across multiple objects;
- resolve an identifier, then fetch details;
- paginate until a condition is satisfied;
- explain a not-found or permission error;
- reject a destructive request when mutation is out of scope.

Avoid questions answerable from tool names alone. Include enough ambiguity to
exercise tool descriptions, but only one defensible final answer.

## Store cases

JSON keeps a Rust runner simple:

```json
[
  {
    "id": "most-closed-issues",
    "prompt": "Using the configured project MCP server, find which fixture user closed the most issues in March 2024. Return only the username.",
    "expected": "sarah_dev",
    "mode": "read-only"
  }
]
```

Never put API keys in fixtures. Pass server credentials through environment
variables referenced by Codex MCP configuration.

## Configure server

Add server with `codex mcp add` or a `[mcp_servers.<name>]` table in
`~/.codex/config.toml`. Confirm effective configuration before evaluation:

```bash
codex mcp list
codex mcp get project-server
codex login status
```

For a stdio server, Codex launches configured command. For remote HTTP servers,
complete required OAuth or bearer-token setup first.

## Run one case

Use an ephemeral, read-only Codex execution and capture only final answer:

```bash
codex exec \
  --ephemeral \
  --sandbox read-only \
  --output-last-message work/eval-most-closed.txt \
  'Using the configured project MCP server, find which fixture user closed the most issues in March 2024. Return only the username.'
```

Use `--json` when runner also needs tool-call events. Do not bypass approvals or
sandboxing for evaluation.

## Batch runner

For repeatable suites, build a Rust binary that:

1. reads cases and expected answers;
2. invokes `codex exec --ephemeral --sandbox read-only` per case;
3. writes each final message to an isolated temporary path;
4. enforces a wall-clock timeout;
5. compares normalized exact answers or typed invariants;
6. writes JSON plus a Markdown summary;
7. exits nonzero when required cases fail.

Do not use an LLM judge for values that can be compared exactly. For narrative
answers, define a JSON Schema with `--output-schema` and assert structured fields.

## Mutation cases

Run mutation tests only against disposable fixture accounts or local emulators.
Each case must name created resources and clean them up. Keep mutation cases in a
separate suite so normal CI can stay read-only.

## Diagnose failures

Classify failure before changing server:

- **selection**: Codex chose wrong tool; improve tool name and description;
- **schema**: arguments were confusing or too permissive; tighten input schema;
- **transport**: process, OAuth, URL, or protocol failed;
- **pagination**: tool response hid cursor or continuation semantics;
- **answer**: tool data was correct but task or expected answer was ambiguous;
- **fixture drift**: source data changed; freeze or rebuild fixture.

Record failing prompt, Codex version, MCP configuration shape with secrets
redacted, server version, final answer, and relevant JSONL events.

## Quality gate

Before release:

- every case runs from clean fixture state;
- read-only suite performs no mutations;
- secrets never appear in logs or reports;
- timeouts terminate hung server processes;
- expected answers were verified independently;
- failure report identifies case and failure class;
- suite returns nonzero on regression.
