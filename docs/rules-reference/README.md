# Rules Reference — Detailed Measurements and Examples

On-demand detail and evidence for portable rules. These docs hold the incident narratives, measured cases, and code samples that support the rules themselves.

## By rule

### Git discipline
- [`git-discipline-incidents.md`](git-discipline-incidents.md) — the 61-commit dead branch, the 11-commit collapse, one session's 29 PRs, the backgrounded commit that shipped an empty branch

### Agent parallelism
- [`agent-parallelism-measurements.md`](agent-parallelism-measurements.md) — file-count split calculations, CPU contention measurements, process counting, disk cleanup

### Testing and gates
- [`testing-gates-detail.md`](testing-gates-detail.md) — code samples for test hangs, module mocks, process-per-test, compile gate phase ordering, vitest timeout collateral

### PR evidence report
- [`pr-evidence-report-examples.md`](pr-evidence-report-examples.md) — HTML/CSS code samples (screenshots, before/after, copy blocks), accessibility requirements, 2026-09-21 convention regression

### Layer boundaries
- [`layer-boundaries-godmodule.md`](layer-boundaries-godmodule.md) — the seam strategy, the 2,445→3,762 line incident, return shapes, the cheap 969→340 example

### Process ownership
- [`process-ownership-incidents.md`](process-ownership-incidents.md) — 2026-09-18 incident (load 92→251, orphan costs), PPID-1 sweep procedure, killing safely

### Timeouts
- [`timeouts-clippy.md`](timeouts-clippy.md) — Clippy's `nonstandard_macro_braces` incident (5.9M calls, 25% runtime, 3,133× speedup), phase-moving consequences

### Estimation
- [`estimation-incident.md`](estimation-incident.md) — 2026-09-01 effortHours incident (18 agents inventing numbers), why confidence scales matter

### Data over binary
- [`data-over-binary-case.md`](data-over-binary-case.md) — the `Online`/`online` case (4 data fixes, 1 infrastructure, 0 app code)

### UI: honest states and copy
- [`ui-remote-states-patterns.md`](ui-remote-states-patterns.md) — RemoteState<T> type definition, courses incident, hand-rolled state gates

## How to use

When reading a portable rule and you want the full incident narrative, measurements, or code samples, refer to the corresponding detail doc here. The rule file itself keeps the principle and a one-line pointer to the detail.

These docs keep the rendered `AGENTS.md` small. Principles are rendered into it and paid for on every session; measured cases, code samples and long narratives are installed to `~/.codex/rules-reference/` and read on demand.
