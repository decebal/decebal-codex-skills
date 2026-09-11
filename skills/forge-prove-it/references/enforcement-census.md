# G1 — the enforcement census

Produces the emitted skill's coverage ledger. **This is the load-bearing phase.** Get it wrong in
the loose direction and the reviewer stays silent about real holes; get it wrong in the strict
direction and it re-reports what a gate already said, which is how a reviewer becomes noise.

Three questions per check, and they are genuinely different:

1. **Does it run?** — its *trigger*
2. **Does it look at the changed path once running?** — its *scan root*
3. **Can it stop a merge?** — its *authority*

Most tooling conflates 1 and 2, and the gap between them is where gates fail silently:
**a check whose trigger is wider than its scan root fires and reports success having looked at
nothing.** Downstream, a green check and a green check look identical.

---

## Rule 0 · Every discovery command is git-driven

`find` and `grep -r` walk worktrees, vendor directories and build output. Measured on one repo:
`find . -maxdepth 4 -name 'Dockerfile*'` returned **23** hits, 10 of them the same files seen
again under `.worktrees/`. The unit graph built from that has five copies of one crate in it.

```
git ls-files '<pattern>'        # not find
git grep -nE '<pattern>' -- '<pathspec>'   # not grep -r
git ls-files --others --exclude-standard   # untracked, when you actually want it
```

**Quote every glob.** zsh expands an unquoted `--include=*.ts` and aborts the whole command with
`no matches found`.

**`git grep` is not GNU grep, and the difference fails silently.** Measured on one file:

```
git grep -cE '^\s+"'        <file>   →  no output       # \s is INERT under -E
git grep -cE '^[[:space:]]+"' <file> →  24              # the same lines, POSIX class
git grep -cE 'name\|version'  <file> →  no output       # \| is a LITERAL pipe under -E
git grep -cE 'name|version'   <file> →  3               # unescaped | is alternation
```

- Under **`-E`**: use `[[:space:]]`, `[[:digit:]]`, `[[:alpha:]]` — `\s`, `\d`, `\b` are inert.
  Alternation is a bare `|`; an escaped `\|` matches a literal pipe character.
- Under **no flag** (BRE): alternation *is* `\|`, and a bare `|` is literal. The two are exact
  opposites, so check which engine the flag selected before trusting a zero.
- `-P` gives PCRE where git was built with it. Gate on it rather than assuming: `git grep -P '' 2>&1`.

Both of the wrong forms above return **empty**, which reads exactly like "nothing found".

**Beware shell shims.** Where `grep`, `find` or `gh` are wrapped by a function or an output-filter
proxy, a probe can be rewritten or rejected and return **empty** — indistinguishable from "nothing
found". One measured case: `find … -not -name '*.sample'` returned `does not support compound
predicates`, and `command find` did **not** escape it; only `/usr/bin/find` did. Where a command's
exact output is load-bearing, use an absolute binary path or a pure-git primitive, and confirm the
output is not truncated before parsing it.

**No discovery zero is final until the pattern has matched something known.** A pattern that
returns nothing because it is wrong reads exactly like a true absence, and the generator will
write that absence into the skill as a fact. Run every pattern against a line you know matches
before believing its zero.

---

## Step 1 · Enumerate every mechanism

### Git hooks — `.git/hooks` is a double false-negative trap

```
git config --local --get core.hooksPath   # REPO state — what the repo itself configures
git config --show-origin --get core.hooksPath   # which file set it, if any
test -x "$(git rev-parse --git-path hooks)/pre-commit"   # is a hook INSTALLED in this clone
```

**`git config core.hooksPath` without `--local` reads merged config** and returns the developer's
*global* setting — machine state, which the generator would then write into the skill as a repo
fact. Worse, a global `core.hooksPath` **silently disables the repo's own `.git/hooks`**, so when
one is set, record it as a finding rather than a configuration detail.

**This installed-hook check is the one place a non-git probe is required**, and its answer is
per-clone. A repo whose enforcement lives in a framework that must be installed (`pre-commit`,
husky and friends) gets a ledger row that says so:

> `installed in THIS clone; per-developer, unverifiable for others`

That is not a blocking-tier row. Treating it as one claims enforcement the repo cannot guarantee.

Measured on one repo: `.git/hooks` held 14 inert `.sample` files and **zero** live hooks, while
`core.hooksPath=.husky` held 43 KB of real ones. A naive listing reports "hooks exist" (the
samples) or "no hooks" (the real ones are elsewhere) — both wrong.

A hook is real only if it is **executable** and **not a sample**:

```
git ls-files '.husky/*' '.githooks/*' | while read -r h; do test -x "$h" && echo "$h"; done
git ls-files 'lefthook*' '.pre-commit-config.yaml' '.overcommit.yml' '.rusty-hook.toml'
git grep -n 'simple-git-hooks\|husky\|lint-staged' -- package.json
git grep -n 'cargo-husky' -- '*Cargo.toml'
```

**Read the config from the tree; use the binary only to expand it.** A committed `lefthook.yml`
with no `lefthook` installed is still the declared mechanism. A missing binary is not evidence of
a missing mechanism. (And `pre-commit` has **no** subcommand that lists hooks — parse the YAML.)

### CI — the tree is authoritative for *declared*

```
git ls-files '.github/workflows/*' '.gitlab-ci.yml' '.circleci/*' '.buildkite/*' 'Jenkinsfile' 'azure-pipelines.yml'
```

**Do not take the declared set from `gh api …/actions/workflows`.** It lists workflows whose file
has been **deleted**, still marked `active`. Measured: 11 registered versus 10 in the tree, the
extra being a workflow with 2 runs ever and no file. The tree is authoritative for what is
declared; the API is authoritative for what has been *observed to run*.

Per workflow, record the triggers:

```
git grep -nE '^on:|^[[:space:]]{2}(push|pull_request|workflow_dispatch|schedule|merge_group):|^[[:space:]]{4}(branches|paths|types):' -- .github/workflows
```

- `on: push` with no `pull_request` does not run on a fork PR.
- `paths:` filters are a trigger — bucket 3 when the remit is touched and the filter missed it.
- **Exclude tag-, schedule- and dispatch-only workflows from the merge-gate surface entirely.**
  They never gate a PR, and an empty `gh run list` for one does **not** mean it is dead.

**GitLab needs its own extraction, or every row lands with a blank trigger** — and a blank trigger
is one G1 forbids delegating to, so the whole ledger collapses to bucket 4:

```
git grep -nE '^(workflow|rules|only|except):|^[[:space:]]+- (if|changes|exists):' -- .gitlab-ci.yml
git grep -nE '^include:|^[[:space:]]+- (local|project|remote|template):' -- .gitlab-ci.yml
```

Follow every `local:` include and extract its rules too. Where an `include:` names a `project:`,
`remote:` or `template:`, the declared set is **not resolvable from the tree** — emit
`UNVERIFIED: pipeline assembled from a remote template; declared set not resolvable here`.

That is also the bound on "the tree is authoritative for declared": it holds where the CI config
has no remote include, and GitLab's `include:` is the common case where it does not.

### Task runners — where the orphans hide

```
git grep -nE '^[a-zA-Z0-9_-]+:' -- Makefile '*.mk' 'mk/*' 'make/*'   # drop names starting with '.'
git grep -nE '^include ' -- Makefile '*.mk'                          # follow one hop
git grep -nE '^\.PHONY:' -- Makefile '*.mk'                          # the declared list, when present
git grep -nE '^[[:space:]]{2}[a-zA-Z0-9:_-]+:' -- Taskfile.yml
git grep -nE '^[a-z0-9_-]+:' -- justfile Justfile
git grep -n -A40 '"scripts"' -- package.json
git ls-files 'nx.json' 'turbo.json' 'mise.toml' '.mise.toml'
```

For turbo the flag is `--dry-run=json` (**not** `--dry=json`); the wrong flag exits non-zero and
reads as "turbo not configured".

## Step 2 · Trigger vs scan root

**The trigger** is external: a hook's path filter, a workflow's `paths:`, a pre-commit `files:`
regex, a staged-file classifier, a task's conditional.

**The scan root** is internal, and it hides in five places — a code-only grep misses four:

**1. In the checker's code:**

```
git grep -nE 'WalkDir|glob\.scan|os\.walk|rglob|readdir|ROOTS?|SOURCE_DIRS|Path::new|filepath\.Walk|chdir' -- <checker>
```

| Where else | How to find it |
|---|---|
| **in a config file** | a `roots = [...]` / `include:` key in the gate's own TOML/JSON/YAML |
| **in a hand-maintained file list** | an array of paths enumerated by hand, which silently shrinks relative to the tree |
| **in the invocation's `cwd`** | a `cd` inside the script, or an npm script defined in a sub-package — it never sees its siblings |
| **in the tool's own ignore file** | an ignore entry is a scan root expressed as a negative. One measured case: a formatter config ignoring a whole shipped app, leaving it with no pattern gate at all |

Then compare the two sets:

| Comparison | Bucket |
|---|---|
| trigger matches the path **and** scan root contains it | 1 · **DELEGATED** |
| trigger matches, scan root does **not** contain it | 2 · **fires, partially blind** — the panel reviews it |
| trigger does not match, but the remit is touched | 3 · hand-check, one command |
| no trigger anywhere | 4 · the panel's beat |

**Mismatches run both ways and both are worth recording.** One real gate has been observed
mismatched in **both directions at once**: a package that triggered it and was never scanned
(bucket 2), and a package that was scanned and never triggered it (bucket 3).

**Assert every scan root resolves to at least one tracked file:**

```
git ls-files -- '<root>' | head -1
```

A rule whose root resolves to zero files is **not a passing rule**. One repo's gate config records
two predecessors that printed `not found — skipping` and exited 0 after a rename retired them.

**Where a trigger cannot be resolved** — a dependency closure, a staged-file classifier, a
reverse-dependency graph — say `UNKNOWN` and **never delegate to it**. A wrongly-DELEGATED check
structurally bars the panel from a whole area, which is worse than no ledger. Where the repo ships
a planner that would resolve it, use it **only if a staleness receipt proves it current**, and
**never build it** — a stale planner produces exactly that wrongly-DELEGATED list.

## Step 3 · The orphan sweep

A check declared somewhere and invoked by nothing. It is indistinguishable from one that passes,
which is what makes it worth a dedicated sweep rather than a glance.

**It is transitive reachability to a fixpoint, not a grep.** A naive single-pass sweep measured
**17 false positives out of 22 rows**, in two distinct ways:

- **Aggregator chains.** `check:button-size` has no direct call site; it is reached as
  pre-push → `check:css-quality` → `check:button-size`. Reachability must close over the graph.
- **Alias versus implementation.** `check:remote-recovery` has zero call sites *by that name*,
  while its implementation path is invoked directly by both the hook and the CI gate script.
  **Resolve every node to both its name and its implementation path**, and treat a hit on either
  as a call site.

```
# roots: what a hook, a PR-triggering workflow, or a task entry point invokes
# edges: for each declared target, what its body invokes (by name OR by implementation path)
# orphans: declared targets not reachable from any root, after closing the graph
```

**Exclude a target's own declaration line from its call-site set.** Otherwise every target matches
itself and the sweep reports zero orphans — which in a Makefile- or justfile-driven repo is the
entire check list reported clean.

**Implicitly-invoked targets are not orphans.** These have zero textual call sites *by
construction*, because a framework invokes them by name. Put them in a separate `IMPLICIT` bucket
rather than reporting them:

| Kind | Examples |
|---|---|
| package-manager lifecycle scripts | `prepare`, `postinstall`, `prepublishOnly`, and any `pre*`/`post*` paired with a real script |
| framework-invoked hook ids | a `pre-commit` config's `id:` entries |
| build-system special targets | `.PHONY`, `.DEFAULT_GOAL`, and any name beginning with `.` |

Do the same sweep for **test suites**: a suite in a directory no runner selects, a target excluded
by a default filter, a profile passed by no command.

**Also check the reverse — two consumers that disagree.** Compare id **strings**, not their
meanings. A classifier emitting `SCOPE-api` while the hook tests for `SCOPE` means
those steps never ran once, and both files read as correct in isolation.

## Step 4 · Authority — read it, never assume it

The question is not "is there CI". It is **"can a red run stop a merge"**, and the answer
**inverts the delegation calculus**. Copy another repo's answer and you either tell the reader
that a genuinely blocking check "is never an answer", or you silence the panel on everything.

```
gh repo view --json nameWithOwner,defaultBranchRef          # never assume the trunk is main
gh api "repos/OWNER/REPO/branches/TRUNK/protection"
gh api "repos/OWNER/REPO/rulesets"
gh api "repos/OWNER/REPO/branches/TRUNK" --jq .protected     # the fallback that survives all cases
```

**Two different 403s mean opposite things — one is definitive, one is no evidence at all. Never
collapse them, and read the response body rather than the status:**

| Response | Means |
|---|---|
| `200` with `required_status_checks` | **those** checks block. List them by name; everything else is advisory |
| `404` on `/protection` | the branch is simply unprotected. Nothing blocks |
| `403` "Upgrade to GitHub Pro or make this repository public" | protection is unavailable **by plan, not by misconfiguration**. Nothing can be required, and no amount of CI changes that — **definitive** |
| `403` "Resource not accessible by integration" / "Must have admin rights" | **UNKNOWN.** The token lacks admin read; this is not evidence either way |
| no `gh`, or another forge | `UNKNOWN` |

**Matrix jobs break the name join.** A job named with a matrix expression expands to one check-run
name per shard, and a job with no `name:` key surfaces under its job **key**. An exact-string join
reports every matrixed job as declared-but-never-invoked. Extract both forms.

**Non-Actions CI reports as legacy commit statuses**, not check-runs — querying only check-runs
misses every external provider.

**GitLab CAN answer the blocking question**; do not file it with the others:

```
glab api projects/:id --jq '{pipeline: .only_allow_merge_if_pipeline_succeeds, approvals: .approvals_before_merge}'
```

With no `glab` on `PATH`, keep `UNKNOWN` but name that command as the one that would settle it.

**CircleCI, Buildkite and Jenkins cannot answer it from anything in the repo** — blocking lives in
the forge's required-checks setting, and for Jenkins partly outside the repo entirely. Say
`UNVERIFIED:` with the command that would settle it. **Never infer authority from the presence of
a workflow file.**

### What the answer changes

Where nothing can be required:

> the pre-push hook is not belt-and-braces — it **is** the braces.

A bypass flag is then a real hole rather than a convenience, a gate moved from the hook to CI-only
is a **loss** rather than a relocation, and the standing footer is necessary:

> *"CI would have caught it" is never an answer here.*

Where checks **are** required, name them; the emitted skill delegates to those and to nothing else.

## Step 5 · Write the ledger

One row per check: id · trigger · scan root · bucket · authority · and, for bucket 2, the exact
paths that trigger it and are never scanned.

Plus the two standing lines, printed on **every** review the emitted skill produces:

```
{{authority_footer}}
{{orphan_list}}   — declared, reachable from nothing
```

**Emit greppable symbol and id names, never line numbers.** A line number is a time bomb: in one
repo two rule docs disagree about the same code today, and one names a test symbol that was
renamed. The emitted skill carries the name plus discipline rule 7 — re-grep before resting a
finding on any citation.

---

## Four things the census must not do

| Not done | Why |
|---|---|
| Run the gates | the census reads configuration. Running them is the reviewer's job to *delegate*, and this skill starts no build |
| Trust a gate's own success | a gate that scanned an empty set exits 0. Bucket 2 exists because exit codes are not evidence — check the artifact, not the code |
| Record a count of violations | a stale inventory is the defect the emitted skill exists to catch. Record the **configuration**; the reviewer measures the **delta** itself, at base and at HEAD, every run |
| Port another repo's blind-spot list | those rows are the **output** of this procedure, not an input. The procedure is the asset; someone else's four rows are yesterday's results |
