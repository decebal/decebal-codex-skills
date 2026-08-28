# UI: honest states and honest copy

Three rules that regress in every codebase, each with the incident that produced
it. None of them are style preferences.

## 1. Never render a raw payload to a user

No JSON, no serialized fragments, no truncated identifiers (`cq-01krghv5kh27…`), on
**any** surface a user can reach. Someone reviewing a record cannot act on half an
object, and a dump is not a worse summary — it is the absence of one.

**The fallback must never be "print it anyway."** State what is known and admit
what is not: `Finished — no summary available` beats `{"disposition":"needs_rev…`.

The pattern that works: a **pure function** that parses the payload, composes a
sentence, and gates its own output on an `isProse` predicate. Because a verdict is
built from a fixed vocabulary, a brace, bracket or quote in the result is *proof*
that a payload leaked — and the honest fallback wins.

Drop identifiers a human cannot use. A label with a space or a dot
(`Acme Pharmacy`, `example-pharmacy.com`) is a name and can render; an opaque id
supports no decision.

## 2. User-facing copy is plain English

- **Name the noun behind every count.** `1,936 sites waiting`, never `1936 queued`.
- **Never leak scheduler or engine vocabulary** — `claimed`, `free slot`, `dead
  letter`, `redrive`, `drain`, `enqueue`, `stalled`, `disposition`, `step_run`. The
  reader operates the product, not the runtime.
- **Name the actor in third person** for anything the reader cannot do — "your
  admin puts you on one", not "add yourself to a queue".
- **Never instruct an impossible action.** An empty state telling the reader to do
  something they lack permission for is worse than no empty state at all.

## 3. Status belongs to the thing it describes

Never a general banner over unrelated items. A board-level `STALLED` banner once
sat above two queues that were merely empty — the healthiest state a queue can be
in — and so asserted something **false** about both. Health is a property OF a
queue: derive it per item and render it beside that item's own name.

Same idea for liveness: streaming text, "Generating…" pulses and typing dots claim
tokens are arriving *now*. Gate them on the thing actually streaming, at the prop
site as well as the render site — a history card that inherits a stream renders a
pulse over nothing.

## 4. A screen that cannot establish its data must carry a way out

Three facts, kept apart, because a reader acts on each differently:

| The screen knows | State | Affordance |
|---|---|---|
| We asked, here is the answer | `ready` | the content |
| We asked, the answer is authoritatively nothing | `empty` | **no retry, ever.** Name who can change it, third person. A re-check is allowed only where the answer can change without the reader acting (an admin granting access in another window) |
| We could not ask, so we do not know | `unreachable` | **a retry, always.** It RE-FETCHES — never `location.reload()`, which in a desktop webview tears down the whole app to fix one panel |

Collapsing the last two is the defect. A shipped build showed *"Your courses will
appear here once it can reach the server again"* over an empty list. Nothing
re-checked. The sentence promised a recovery the code never performed, and there
was no control to trigger one.

**Make the type the enforcement.** Put `RemoteState<T>` and its renderer in the one
package every consumer depends on, so shared components receive the state and each
adapter constructs it:

```ts
type RemoteState<T> =
  | { kind: "ready"; data: T }
  | { kind: "empty"; message: string; actor: string; recheck?: () => void; recovery?: never }
  | { kind: "unreachable"; message: string; recovery: { label: string; retry: () => void } }
```

- Omitting the recovery on `unreachable` is a **compile error** — `recovery` is
  required.
- Putting a retry on an authoritative `empty` is a **compile error** too —
  `recovery` is typed `never`. Both directions on purpose: made only-one-way,
  authors relabel an empty as unreachable to satisfy the compiler.

**Back it with a gate** for components that hand-roll their own `error` flag and
never adopt the type: `gates/ts/check-remote-recovery.ts` fails on a failure branch
whose body offers the reader nothing. It deliberately never reads an **empty**
branch — whether "nothing is turned on" deserves a button is the semantic call the
type carries, and a gate that guessed would be wrong in the direction that adds
impossible actions, which rule 2 forbids.

## The meta-rule

Most UI standards have no CI gate; a reviewer is the only check. So write each rule
**with the incident that produced it**. A bare prohibition gets rationalized away by
the next person under deadline; a prohibition with a cost attached does not.
