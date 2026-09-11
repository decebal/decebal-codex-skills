# Host differences — what changes between the emitted copies

`--host both` writes the same census into two skills. Most of the content is identical, because
the census is a fact about the repository and not about the agent reading it. Four things are
not, and each one silently degrades the emitted skill if it is copied across unchanged.

Determine the host from what exists, and say which you picked:

```
ls -d .claude/skills .agents/skills ~/.codex/skills
ls CLAUDE.md AGENTS.md
```

Probe the **skills directory**, not the dot-directory above it. The two hosts do not share one:
one scans `.claude/skills/`, the other `.agents/skills/` for repo scope and `~/.codex/skills/` for
user scope. A skill written to a plausible-looking sibling is never loaded, and nothing reports it.

Neither present → ask. Both present → `--host both`.

---

## 1 · Fan-out — the only structural difference

The emitted Phase 2 runs one refuter per triggered lens, and Phase 3 runs counter-refuters. **A
host that cannot spawn subagents must not be handed a skill whose phases assume it can** — it
will either serialise them into one context that blends the lenses together, or quietly skip the
ones it cannot run.

| Fan-out available | Phase 2 / 3 |
|---|---|
| **unconditional** — the host always has subagents | parallel, one per lens. Each gets the diff **inlined in its prompt** — never "go run the diff yourself", which is N agents paying a round-trip for bytes the orchestrator already holds. |
| **conditional** — subagents exist sometimes | delegate one per lens *when available*, and **fall back to the row below when not**. The emitted skill must handle both, because the fallback is silent otherwise. |
| **none** | **sequential passes in one context, one lens at a time, each writing its findings to a file before the next begins.** The file is what keeps the lenses from blending — a lens that can see the previous lens's reasoning stops being an independent attack, and independence is the only thing that makes counter-refutation mean anything. |

**The emitted skill must print which of the three actually happened.** A coverage claim resting on
"six lenses ran independently" is a false claim when the fan-out silently collapsed to serial, and
the reader has no way to tell:

```
prove-it · <branch> · <range> · 6 lenses, parallel
prove-it · <branch> · <range> · 6 lenses, sequential (no fan-out available)
```

Write the *capability test*, not the host name. A host's fan-out can change between releases, and
a skill that hard-codes "this host has subagents" is wrong the day that changes.

**What must not change with the host:** counter-refutation. Phase 3 is what enforces "a kill needs
evidence too", and dropping it because the host is single-context converts every unkilled finding
into a hand-check — which routes *more* work to the human, not less.

## 2 · The tools the emitted skill may name

The generated skill cites commands in its hand-checks and its probes, and a hand-check citing a
tool the host cannot run is a dead row.

- Cite **shell commands** wherever possible. They are the portable floor and every host has them.
- Cite a **host-specific tool** only when the census actually found it configured, and name it as
  optional: *"if `<tool>` is available…"*. An unavailable tool is a connection failure, never
  evidence of absence — that is discipline rule 11, and it applies to the generator as much as to
  the emitted skill.
- **Never cite a tool because the other host has one.** This is the most common way the two
  copies drift into fiction.

## 3 · Posting the review

Posting is a property of the forge, not of the host: it needs a code host with a review API and
the credentials for it.

| Condition | Emitted behaviour |
|---|---|
| `gh` is on `PATH` **and** `origin` is GitHub | emit the posting section: one review, inline comments anchored on lines the diff actually adds on the RIGHT side, body carrying the claims verdict and the whole `VERIFY BY HAND` block. Check for an existing review at this `commit_id` first — a tool that duplicates itself on every re-run is a tool people mute. |
| any other forge, or no CLI | emit terminal-only, and say so in one line. Do not emit a posting section that cannot run. |

Two rules that hold wherever posting is emitted, and both were learned by posting:

- **Never `APPROVE`.** A panel that could not refute a claim has not approved anything; approval
  is a person's call.
- **Check the author first.** Where the review account is the PR's own author, GitHub rejects
  `REQUEST_CHANGES` with a 422 and **the whole call fails**. Fall back to `COMMENT` and say in the
  body that the downgrade is the API's rule, not the verdict's. On a repo where most PRs are
  self-authored this is the common path, not the edge case.

## 4 · Frontmatter and metadata

Same body, different wrapper. Write whichever the target repo's own gate requires, and **read the
gate rather than copying a sibling** — a sibling can be grandfathered.

| | Claude | Codex |
|---|---|---|
| `name` | matches the directory | matches the directory; lowercase, digits and hyphens only, no leading/trailing hyphen, no `--` |
| `description` | single or folded scalar | **single-line quoted.** A folded scalar (`>-`) parses to two characters under a line-oriented metadata check, so the description passes the gate while carrying nothing |
| angle brackets | fine | **forbidden anywhere in `description`.** That bans `->`, a generic type written with brackets, and every `<placeholder>`. Write the arrow as `→` and the placeholder in words |
| extra files | none required | `agents/openai.yaml`, with `display_name`, a `short_description` of **25–64 bytes**, and a `default_prompt` containing the literal `$skill-name` — no braces, or the check misses it |

**Measure the length in bytes, not characters.** An em dash, an arrow or a curly apostrophe costs
2–3 bytes each, so a 64-character sentence with two em dashes is 68 bytes and fails a 64-byte
ceiling. And a value in single quotes is not unquoted by a parser that strips only double quotes —
the quotes then count toward the length.

```
printf '%s' "<the value>" | wc -c
```

**Check the plugin manifest for a count.** Where a repo's manifest describes itself as "N skills",
adding one makes that number wrong. It is exactly the stale-inventory defect the emitted skill
exists to catch, so fix it in the same change.

---

## Keeping the two copies in step

The census is one run; the copies are two renderings of it. When the enforcement surface moves,
re-run `--update` for **both** hosts in the same change.

A fact that is true in one copy and stale in the other is worse than a fact that is stale in
both: the reader who checks the fresh one learns to trust the skill, and then acts on the stale
one.
