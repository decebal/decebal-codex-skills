# forge-prove-it references

Seven files. Four are the census — how a fact gets established. One is the shape the established
facts get written into. Two are cross-cutting.

| File | Phase | What it holds |
|---|---|---|
| [`enforcement-census.md`](enforcement-census.md) | G1 | hooks, CI, trigger vs scan root, the orphan sweep, blocking authority. **The load-bearing one.** |
| [`structure-census.md`](structure-census.md) | G2 + G3 | units, planes, layers, canonical seams, registration chains; then every ceiling, allowlist and suppression |
| [`incident-mining.md`](incident-mining.md) | G4 | turning a repo's own prohibitions, reverts and ADR consequences into lens probes |
| [`interview.md`](interview.md) | G5 | the seven questions a tree genuinely cannot answer, each with an honest default |
| [`emitted-skill-template.md`](emitted-skill-template.md) | G6 | the portable shape: lenses, phases, discipline rules, output block order |
| [`self-audit.md`](self-audit.md) | G7 | the five checks over the files just written, and why marking beats deleting |
| [`host-differences.md`](host-differences.md) | G0 + G6 | what changes between two host copies of the same census — fan-out, tools, posting, frontmatter |

## Reading order

Writing a prove-it into a new repo: `enforcement-census` → `structure-census` →
`incident-mining` → `interview` → `emitted-skill-template` → `self-audit`.
`host-differences` matters only when emitting for more than one host.

Changing how the generator works: start at `emitted-skill-template`, because it defines what the
census has to be able to fill. A census step that fills no slot is waste; a slot no census step
fills is a guess waiting to happen.

## The rule all seven share

> Every line written into the generated skill is either a mechanism that holds in any repository,
> or a fact this run observed, carrying the command that observed it.

An unfillable slot ships as `UNVERIFIED:` with the command that would settle it. Never as a
plausible default — a generated reviewer citing a gate that does not exist teaches its readers to
disbelieve it, and they are right to.
