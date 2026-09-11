# G2 + G3 — the structure and guard census

Fills the emitted L1 (placement), L2 (reuse) and L3 (guard-weakening).

Two rules from `enforcement-census.md` apply to every command here and are not repeated at each
step: **every discovery is `git ls-files` / `git grep`, never `find` / `grep -r`** (worktrees and
build output otherwise multiply the answer), and **no zero is final until the pattern has matched
a line you know it should match**.

**Directory names are not evidence.** A folder called `domain/` proves a convention was intended,
never that it holds.

---

# G2 · Structure

## 1. Units — from the manifests that exist

```
# Cargo — prefer the manifest to `cargo metadata`, which takes the package-cache lock
git grep -n -A40 '^\[workspace\]' -- Cargo.toml

# npm / bun / yarn / pnpm
git grep -n -A20 '"workspaces"' -- package.json
git ls-files 'pnpm-workspace.yaml'

# Go — list the ROOT manifest explicitly; a '**/x' pathspec never matches a root-level x
git ls-files 'go.work' 'go.mod' '**/go.mod'

# Python
git grep -nE 'members|packages|tool\.uv\.workspace' -- pyproject.toml

# Others: composer.json repositories · mix.exs apps_path · pom.xml modules · settings.gradle
```

**A `**/x` pathspec does not match a root-level `x`** — it requires a literal slash. A repo with a
single top-level module discovers **zero** units if the root manifest is not listed separately.
Apply the same fix to every `**/`-only pathspec you write.

Record each unit's name, path, and manifest line. Nothing else yet.

**Look for config *absence* as well as presence.** Every other discovery step in this census
finds a unit by something it *has*; a unit with no lint config, no ignore entry and no CI job is
found by none of them.

Record what the four commands establish, and nothing beyond it:

```
git ls-files '<unit>/.eslintrc*' '<unit>/*.toml' '<unit>/tsconfig*'   # its own config
git grep -n '<unit>' -- <root ignore files>                          # named in an ignore list?
git grep -n '<unit>' -- .github/workflows Taskfile.yml Makefile package.json   # invoked anywhere?
git ls-files '<unit>' | wc -l                                         # is it real
```

> `unit <name>: manifest at <path>; no lint config, no ignore entry, no CI job —
>  established by the four commands above`

**What the unit is *for*, and whether it matters more than the others, is an interview question,
not a census output.** Ranking a unit you have not looked inside against surfaces you have not
measured writes two unobserved facts into the emitted skill.

## 2. Planes — where each unit runs

```
git ls-files 'Dockerfile*' '**/Dockerfile*' 'fly.toml' 'vercel.json' 'netlify.toml' 'serverless.yml' 'Procfile' '*.tf'
git grep -nE 'bundle|resources|externalBin|files|include' -- <the app/packaging manifest>
```

Classify every unit: **shipped to the user** · **runs on our servers** · **build-time or dev-only**
· **published as data**.

The fourth is the one that matters and the one no command finds. A repo with a per-tenant rules
pack, a prompt catalogue, a flag service or a config document store has a data plane, and *"is
this a binary fix or a data fix?"* is then the highest-value question L1 can ask.

**If there is no data plane, drop L1's triage step entirely — do not translate it.** A probe with
no local referent matches nothing forever and teaches the reader that the lens is empty ritual,
which reads exactly like a passing check. Dropping rows with no local equivalent is mandatory,
not optional.

## 3. Layers — only if declared

Look for an **assertion**, not a convention:

```
git ls-files '*.eslintrc*' 'eslint.config.*' '.dependency-cruiser.*' '.importlinter' 'import-linter.ini'
git grep -ln 'architecture\|layer\|boundary' -- '*test*' '*.toml' '*.json'
git grep -n 'no-restricted-imports\|forbidden\|layers' -- <the lint configs found>
```

| Found | Emit |
|---|---|
| an architecture test, import lint, or layer gate | the direction **it** asserts, cited to its file, plus its current ceilings |
| an ADR or rule doc describing one, unasserted | the direction as **convention**, quoted; L1 reports a crossing as `sand` |
| nothing | `UNVERIFIED: no declared layer boundary found`, and **L1 emits without a layer rule** |

**Never synthesise a boundary from the tree.** A generated layer rule nobody agreed to is a
reviewer arguing with its own author, and it is the fastest way to get the whole skill
disbelieved on its first run.

## 4. Canonical seams — frequency, then confirmation

Find candidates by **import frequency** — and use the command for the language, not one regex for
all of them. A single `^(use|import|from)` pattern returns **zero** on Go, whose imports are
indented inside a grouped block, and collapses to one useless `import` bucket on TypeScript, whose
module path is at the *end* of the line:

```
# Rust
git grep -hoE '^use [A-Za-z0-9_:]+' -- '*.rs'

# TS / JS — match the path tail, not the `import` head
git grep -hoE "from ['\"][^'\"]+['\"]" -- '*.ts' '*.tsx' '*.js'

# Go — grouped imports are indented string literals; the single-line form is separate
git grep -hoE '^[[:space:]]+"[^"]+"' -- '*.go'
git grep -hoE '^import "[^"]+"' -- '*.go'

# Python
git grep -hoE '^(from|import) [A-Za-z0-9_.]+' -- '*.py'
```

Then, in each case: `| sort | uniq -c | sort -rn | head -40`, and drop the external packages.

**Run each of these against a line you know should match before trusting its output** — A4 in
`self-audit.md` requires it, and both defects above are exactly what it catches.

An in-repo module imported by many files across several units is a candidate. Then read **only its
header**:

| Language | The header | The export-list trap |
|---|---|---|
| Rust | the `//!` module doc | `^pub ` under-counts — a crate-internal seam is `pub(crate)`. One measured file exported eight items and matched zero |
| TS / JS | leading block comment + exports | `^export ` misses `export * from` |
| Python | module docstring + `__all__` | `^def ` misses decorated functions |
| Go | the package comment | `^func ` misses methods with receivers |
| Ruby / Elixir | the class comment / `@moduledoc` | — |

**Frequency proposes; the header or the author confirms.** A module imported everywhere may be a
utility grab-bag, and writing it up as canonical sends new code into a dumping ground. Ties go to
interview Q2; an unconfirmable candidate ships `UNVERIFIED:`.

Record for each seam: path, the responsibility from its header, and **the reinvention shape** —
what someone writes instead when they do not know it exists. That shape is L2's grep.

## 5. Registration chains — what fills L5

**Trace from an identifier, never from prose.** Documentation describes the chain someone
remembered, and the hop it forgot is by definition the hop people forget. One measured case: the
repo's own instructions described a four-step chain; grepping a single real command name returned
**five** files, because the handler implementation and its router registration are separate hops.

Find the chains from co-change:

```
git log --format='%h' --since=12.months -- <a feature directory> \
  | while read -r c; do git show --name-only --format= "$c"; done \
  | sort | uniq -c | sort -rn | head -30
```

Files that co-change far more often than they change alone are one chain. Then confirm by picking
**one real feature name** and grepping it across the repo — the file set that comes back *is* the
chain.

| Shape | Grep |
|---|---|
| a registry or dispatch table | `match`, `switch`, a map literal keyed by name, a `register(` call |
| an enum ↔ wire-string map | a derive/decorator with a rename rule — **grep the wire string, not the identifier** |
| a route table | route decorators, a router file |
| a DI container | `bind(`, `provide(`, `singleton(` |
| an ordered migration list | a sequenced directory, or an array of them |
| an event crossing languages | the same quoted literal in two ecosystems |

**Exclude test paths before ranking anything by string-literal density.** Measured: the top four
hits by that metric were a constants file and three test files carrying fixture payloads — a
generator ranking blind nominates a test file as the command registry.

For each chain record **every step in order**, and **which step is checked and which is not**.
A chain where step 1 alone is green is exactly the silent failure L5 exists to catch — and that
fact comes from G1's ledger, not from the chain.

---

# G3 · Guards

## 1. Find them

```
git grep -nE '(MAX|LIMIT|CEILING|THRESHOLD)[A-Z_]*[[:space:]]*[:=]' -- '*.rs' '*.ts' '*.py' '*.go'
git grep -nE 'ALLOWLIST|ALLOW_LIST|EXCEPTIONS|LEGACY|GRANDFATHER|IGNORE_LIST|baseline' -- <gate + test pathspecs>
git ls-files '.eslintignore' '.prettierignore' '.semgrepignore' '.trivyignore' \
             '.golangci.yml' '.golangci.yaml' 'ruff.toml' '.ruff.toml' \
             'mypy.ini' 'setup.cfg' '.flake8' 'tox.ini' '.rubocop.yml' 'clippy.toml'
git grep -nE 'per-file-ignores|ignore_errors|ignore_missing_imports|exclude|allow' -- pyproject.toml
git grep -nE 'timeout|max-warnings|fail-under|threshold' -- <hook and CI pathspecs>
```

Plus the **suppression comments**, a guard by another name: `#[allow(`, `eslint-disable`,
`# noqa`, `# type: ignore`, `@SuppressWarnings`, `//nolint`, `# rubocop:disable`,
`pragma: no cover`.

## 2. Record each one

| Field | Why |
|---|---|
| file and key | so L3 can pair `+`/`-` tuples by key |
| value today | the baseline — a census fact **with its command**, never a count the emitted skill re-asserts |
| closed-and-empty? | a first entry in an empty allowlist is the strongest single signal any prove-it has |
| computed from the tree? | then L3 must **do the arithmetic itself**, every run |

**If a ceiling is computed, read the measure — do not assume it.** One repo's line ceiling counts
production lines *after stripping inline test modules by brace depth*; a naive `wc -l` over-counts
every file that has them, and every resulting finding is confidently wrong. Where the measure
cannot be reimplemented at all — coverage percentages, dependency counts — **route it to a
hand-check rather than estimate it.**

**And check the ratchet in both directions.** A one-directional reimplementation recreates the
incident the ratchet was written to stop: a file that shrinks below the ceiling while keeping its
allowlist entry leaves the inventory rotting back into fiction. One measured entry recorded a file
at 575 lines while it stood at 4,198. The rule is *"remove the entry"*, not just *"do not exceed
the ceiling"*.

## 3. The direction rule — put it in the command

`grep -E '^[+-]'` matches both directions identically. **Pair the tuples by key and compare the
values, or do not ship the detector.**

```
git diff -M <base>..HEAD -U0 -- <guard file> | grep -E '^[+-]'
```

**Sign the comparison per ratchet kind — it inverts.** A *longer allowlist* is weaker; a *longer
trigger glob list* is stronger. A detector treating "the list grew" as weakening everywhere
reports a widened trigger as a regression.

- a key on **both** a `-` and a `+` line → report only where the value loosened
- a key on a `-` line alone → a deletion. Tightening if it is an allowlist entry; **loosening if
  it is a gate id.** Say which.
- a key on a `+` line alone in a **closed and empty** allowlist → the strongest signal in the lens

**Run any comparator against a known-weakening fixture before trusting it.** Its failure mode is a
clean-looking pass: one measured comparator printed nothing for every input because a join emitted
`key=old=new` where the parser expected a space, and it reported "no ratchet changed" in perfect
silence.

## 4. Invocation flags — the class that costs the most

A gate that still runs and no longer asserts. Check it against the Step-1 enumeration in G1: if no
check there compares invocation flags across a diff, this class is unwatched here, and say so with
that enumeration as the evidence.

```
git diff -M <base>..HEAD -- <hook + CI + task pathspecs> \
  | grep -E '^[+-][^#]*(--features|--all-targets|--workspace|--strict|-D warnings|--max-warnings|passWithNoTests|--no-verify|continue-on-error|\|\| true|if: false|timeout|--shard|hooksPath|skip|exclude)'
```

**The `[^#]*` is load-bearing.** Without it the detector matches prose in comments — measured: 5 of
24 hits were comments, one of them a comment asserting the *opposite* of what the grep implied.
The detector inverts on its own evidence.

| Lost or added token | Consequence |
|---|---|
| a required feature flag dropped from a test invocation | the runner **silently skips** targets whose requirements are unmet, and prints green |
| a lint's "all targets" scope narrowed | whole categories of file go unlinted |
| `continue-on-error`, `\|\| true`, `if: false` | the step is advisory — where nothing can be required, it enforces nothing |
| a "pass with no tests" flag added | a suite whose filter matches nothing passes instantly |
| the hooks path pointed at an ignored directory | git runs no hook at all |

## 5. A moved file leaves a named-path list

**This weakens a gate with a zero-line diff to the gate's own config** — invisible to any grep of
the gate. It requires intersecting renames against every discovered allowlist:

```
git diff -M --name-status <base>..HEAD | awk '$1 ~ /^R/ {print $2, $3}'
```

For each, ask whether the **old** path is named in any by-name list — a roots array, a files
array, an allowlist key, a trigger glob, an ignore file — and the **new** path is not. This is a
repeated real incident: a refactor moves files out from under a roots-based rule, and the gate
scans nothing and exits 0, green forever.

## 6. Test weakening

- a skip or ignore **added** — read the reason. An external resource is convention; an unlanded
  fix is a finding.
- assertion arithmetic, **only** where the test file's production counterpart also changed.
- a suppression comment added on the feature's own path.

## 7. Silent success

```
git diff -M <base>..HEAD -U3 | grep -nE '^\+.*(unwrap_or_default|\.ok\(\)|catch\s*\{\s*\}|except:\s*pass|_ = err|rescue nil)'
```

Only a finding when the **same hunk** has a `-` line carrying an error propagation.

**Known false positive: a code move.** An extracted function ends in a bare success while the
deleted body propagated, so any extraction satisfies both sides by construction — and extraction
is usually the repo's *prescribed* way off a size ceiling. Confirm the `+` and `-` lines are not
the same statements at a new address before reporting.
