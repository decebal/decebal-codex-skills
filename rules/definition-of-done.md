# Definition of done — no arbitrary stops

A feature is finished when it **works end-to-end**, not when a checklist turns
green. Drive to that bar; do not invent stopping points before it. "Keep going" or
"work the ready tasks" means finish the whole feature — not the next task, not a
convenient milestone.

- **Complete = works end-to-end.** A human can do the whole job the feature
  promises, through the real UI or flow, start to finish. The acceptance test is
  the task *actually working* — verify it end-to-end (run the flow, or add a
  cross-layer integration test that exercises it), not "the epic's children all
  show `done`". Task state, tests passing, and code pushed are *bookkeeping*, not
  proof the feature works.
- **Size is never a signal.** PR size, diff size, stack depth, and number of open
  PRs are NOT completion criteria and NEVER a reason to pause, split, or "merge the
  base first". A feature ships as one complete PR and merges when the FEATURE is
  done, however large. Do not editorialize about size — ever.
- **Discovered work is in scope.** Work you find while building (a follow-on, a
  missing layer, a gap) is part of THIS feature's completeness — finish it here. Do
  not spin it out into a separate task so you can declare the current one "done".
  Scope grows to whatever end-to-end actually needs.
- **A block is not a stopping point — route around it.** If one slice is blocked on
  something outside your control (an upstream library not yet released, another
  team, a credential), exhaust every alternative path to end-to-end before
  stopping. Real example: an ingest wiring blocked on an importer that wasn't
  merged yet — the route-around was to gate the *existing* producer on the queue
  binding. The block earns a check-in only after the routes-around are gone.
- **Stop only when truly blocked or genuinely ambiguous.** Two legitimate
  mid-feature check-ins: (1) you need a decision, credential, or access you cannot
  get yourself; (2) what "complete" means for THIS feature is genuinely ambiguous
  and guessing risks building the wrong thing. Everything else — "this is large",
  "good checkpoint", "should I continue", "should I merge the base first" — is an
  arbitrary stop.

## Task bookkeeping

- **Never close a task with unchecked acceptance criteria.**
- **A task is done when the work is pushed, not when the PR merges** — merging is
  someone else's action and blocks nothing.
- **Zero successes is not evidence of uselessness.** A path that has never
  succeeded may be measuring its own breakage, not its own irrelevance. Find out
  which before deleting it.
