# Designing Skill Evaluators

Autoresearch is only as honest as its evaluator. Freeze this contract before
recording baseline.

## Corpus

Use representative cases, not one happy-path prompt:

- positive triggers where skill should activate;
- near-miss prompts where it should stay out;
- normal workflows with expected evidence or artifacts;
- failure paths: missing tools, absent files, invalid inputs, and timeouts;
- held-out cases not used to choose each mutation.

Keep fixtures stable through one run. If fixture is wrong, end run, fix it, and
start new baseline.

## Mechanical score

Evaluator must convert observable outcomes into one finite number. Useful terms:

- required artifact or field present;
- authoritative validator pass/fail;
- forbidden fabrication or unsafe action absent;
- exact runtime/test success;
- latency, token, dependency, or instruction-length penalty.

Example higher-is-better score:

```text
100 * passed_required_checks / required_checks
- 5 * fabricated_claims
- 2 * unhandled_failure_paths
- 0.01 * added_instruction_words
```

Weights are domain choices. Record them in evaluator source. Do not use a score
that can improve while critical safety or correctness checks fail; gate those
before scoring.

## Model-based cases

When cases require Codex output, pin model, reasoning level, system context,
tools, and prompt corpus. Run each case multiple times and aggregate with median
or worst-case score. Keep deterministic structural checks outside grader model.
Store raw outputs so score can be audited.

## Command contract

Runner calls executable directly; shell syntax is not interpreted. Use explicit
program and arguments. Evaluator must:

1. return exit code zero only after completing measurement;
2. print final line as `score=<finite number>` or bare finite number;
3. write diagnostic detail before final line or to stderr;
4. avoid editing target skill, evaluator, fixtures, or experiment ledger.

Nonzero exit, timeout, missing score, NaN, or infinity records failed iteration
and cannot replace incumbent.
