# Project-Specific Skills

The skills in this repo are generic — they work in any project. But the highest-
value skills are often the ones that only make sense in *one* project: the exact
release ritual, the durability test that knows your container names, the data-flow
check that walks your specific Docker stack. This guide is about writing those.

## When to create a project skill vs use a generic one

Reach for a **project skill** when the procedure encodes knowledge that is true
only here — ports, container names, service topology, a multi-step release that
touches several repos. Good candidates:

- **Release workflows** — version bump order, which gates run, tagging policy
- **Infrastructure / durability tests** — restart-and-verify, backup-and-restore
- **Data-flow / health checks** — end-to-end walk across a known service stack
- **Domain-specific debugging** — "where do I look when X breaks in *this* system"

Stick with a **generic skill** when the procedure is portable (PRD generation,
brainstorming, skill authoring) — those belong in `~/.agents/skills/` and this repo.

Rule of thumb: if the steps name a port, a container, or a repo, it's a project skill.

## Examples from real usage

| Skill | What it does |
|-------|--------------|
| `chronos-release` | Multi-service version bump → full CI gates → single immutable tag (see [chronis-git-best-practices](./chronis-git-best-practices.md#release-tagging)) |
| `chronos-durability` | Write events → restart the container → verify the events survived |
| `chronos-data-flow` | End-to-end health check across the Docker stack (producer → store → projection → API) |
| `allsource-data-access` | Storage-internals guide: WAL, Parquet, DashMap — where state actually lives |

Each is worthless in another repo and indispensable in its own.

## Pattern: where project skills live

Project skills go in the repo, not your home directory:

```
.agents/skills/<name>/SKILL.md      # project-level — committed, shared with the team
~/.agents/skills/<name>/SKILL.md    # global — your personal, cross-project skills
```

Committing the project skill means every contributor (and every agent) picks up
the same release ritual and the same debugging map. It's documentation that Codex
executes.

## Anatomy of a good project skill

1. **Triggers** — the phrases that should invoke it ("cut a release", "verify
   durability", "the app crashes on startup"). Be explicit; vague triggers don't fire.
2. **Step-by-step procedure** — ordered, verifiable steps. Not prose — commands.
3. **Environment awareness** — the concrete facts that make it project-specific:
   ports, absolute paths, container names, service order, which gate is slowest.
4. **Failure modes** — what breaks, how to tell, what to check first. This is where
   the hard-won knowledge lives; it's the reason the skill exists instead of a README.

A generic skill teaches a technique. A project skill encodes *this system's* muscle
memory so it survives context resets and onboards the next agent for free.
