# Token efficiency

Rule form, for inclusion in a project's agent instructions. Deep dive:
[../docs/token-efficiency.md](../docs/token-efficiency.md).

- **Targeted searches first** — `rg` with specific patterns instead of reading
  whole files. Read only the relevant line range. If `rg` misses, try
  `rg --files` for file patterns. A full file read is the last resort.
- **No whole-file dumps** — never `cat` a large file into context. Print a
  bounded range (`sed -n '120,180p' <file>`) or the `rg -n` hits. For edits use
  `apply_patch`, not `sed -i`. If you must pipe a large command's output, filter
  it to errors and warnings.
- **Cache mentally** — don't re-read files you've already seen this session.
  Reference line numbers from prior reads. Retain: file structure, exports,
  function signatures, import patterns, config values.
- **Concise responses** — state what you'll do, do it, report the result.
- **Batch parallel calls** — read 3 independent files in one message, not three.
  Run independent searches together.
- **Scope strictly** — only what was asked. No added comments, docstrings, or
  refactors to code you didn't change. No speculative improvements. Fewer lines
  changed = fewer tokens on review.
- **Minimal diffs** — prefer a small `apply_patch` hunk over rewriting a file.
  Never rewrite a file to change 3 lines.

## Output filtering (optional tooling)

A proxy that filters command output before it reaches the context window
(compilers, test runners, linters, `git`) saves 60–90% on dev operations. Two
things to know if you adopt one:

- **A summarizing filter can hide a FAILED build.** The summary reads as success.
  Read the tool's own tee log, or the exit code, before believing it.
- **A rewriting hook can break a tool's own flags.** Verify the rewritten command
  is the one you meant; keep an escape hatch, and never disable the proxy globally
  to work around one command.

## Compressed assistant output (optional)

Prose compression (dropping articles, filler and hedging while keeping technical
substance, code and error text verbatim) pairs with output filtering: one shrinks
tool output, the other shrinks the response. Never compress commits, PR bodies, or
user-facing copy. Suspend it for security warnings, irreversible-action
confirmations, and multi-step sequences where fragment order risks a misread.
