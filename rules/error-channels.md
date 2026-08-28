# Error channels — two, never one

Every catch block routes to exactly one of two functions. Never a bare
`console.error`, never an empty `.catch(() => {})`.

| Function | Audience | Shows a toast? | Route |
|---|---|---|---|
| `notifyUser(e, source, phase)` | **the user** | yes, with an action button | alerts stream |
| `logDev(e, source, phase)` | **the dev team** | no | activity/telemetry stream |

- **`notifyUser`** — the user can act on this: a bad API key, a missing tool, a
  budget exceeded. Transform the raw error into plain English with an action
  button.
- **`logDev`** — the user cannot act on this: a fire-and-forget sync, background
  cleanup, a best-effort operation. Routes to the activity feed for team
  observability.

Quick decision: *"Does the user need to see or act on this?"* Yes → `notifyUser`.
No → `logDev`.

## Rules

- **Every error message must be human-readable and actionable.** No user should
  ever see a raw technical error, a stack trace, or a generic "try again".
- **Never write `notify("Could not X. Please try again.")`** — that tells the user
  nothing. Name what failed and what to do.
- **Keep a pattern table** that maps raw backend errors to human messages plus an
  action. When the backend adds a new error type, add a pattern; do not let it fall
  through to the generic branch.
- **Error copy must not loop.** "Reconnect to continue" on a screen with no
  reconnect control is a dead end. Link to the real setup destination, and never
  leak the fact that the limitation is ours.
- **Keep the logger framework-agnostic** — no UI framework, no IPC layer in the
  package. Wire its sink at the composition root on mount; before the sink is
  configured it falls back to the console.

## The gate

Grep for `console.error` and empty catch bodies in a pre-push gate. Both are cheap
to detect and both are how the two channels quietly become zero channels.
