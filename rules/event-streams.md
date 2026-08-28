# Two event streams, strictly separated

Most apps grow exactly two streams and then quietly merge them. Keep them apart.

| Stream | Store | Write function | Shows a toast? |
|---|---|---|---|
| **Activity** (telemetry) | per-entity activity feed | `addActivity(source, phase, msg, entityId)` | **never** |
| **Alerts** (notifications) | notifications | `addToast()` / `notify()` | **always** |

## Rules

1. **Never call `addToast`/`notify` for operational telemetry** — step progress,
   task execution, retries. Use `addActivity()`.
2. **Never call `addActivity` for user-facing messages** — errors, milestones,
   stuck warnings. Use `addToast()`/`notify()`.
3. **Always include the owning entity id** in both the backend event payload and
   the `addActivity()` call, or the feed cannot be filtered.
4. **Deduplicate recurring events.** Anything fired on a timer or in a loop needs a
   backend `HashSet` or a frontend `Set<string>` keyed on the message identity.
5. **Guard the separation with tests.** A pair of tests asserting that
   `addActivity` never touches notifications and that `addToast`/`notify` never
   touch the activity feed costs nothing and catches every future crossing.

Pairs with [error-channels.md](error-channels.md) — same principle, applied to
errors.
