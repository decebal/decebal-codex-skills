# UI remote states — patterns and incidents

Detailed patterns and code supporting `rules/ui-remote-states.md`.

## The courses incident

A shipped build showed *"Your courses will appear here once it can reach the server again"* over an empty list. Nothing re-checked. The sentence promised a recovery the code never performed, and there was no control to trigger one.

This happens when `unreachable` and `empty` are collapsed into one state — the code has no way to distinguish "we asked and got nothing" from "we couldn't ask". **Make the type the enforcement** so this shape becomes impossible.

## RemoteState<T> type

```ts
type RemoteState<T> =
  | { kind: "ready"; data: T }
  | { kind: "empty"; message: string; actor: string; recheck?: () => void; recovery?: never }
  | { kind: "unreachable"; message: string; recovery: { label: string; retry: () => void } }
```

Enforcement by type:

- Omitting the recovery on `unreachable` is a **compile error** — `recovery` is required.
- Putting a retry on an authoritative `empty` is a **compile error** too — `recovery` is typed `never`. 

Both directions on purpose: made only-one-way, authors relabel an empty as unreachable to satisfy the compiler.

### Build states with helpers

```ts
remoteUnreachable({ message, retry })
remoteEmpty({ message, actor, recheck? })
```

Render with `<RemoteDeadEnd state={…} />`. 

## Gate for hand-rolled states

For components that hand-roll their own `error` flag and never adopt the type: `gates/ts/check-remote-recovery.ts` fails on a failure branch whose body offers the reader nothing. It deliberately never reads an **empty** branch — whether "nothing is turned on" deserves a button is the semantic call the type carries, and a gate that guessed would be wrong in the direction that adds impossible actions.

## The meta-rule

Most UI standards have no CI gate; a reviewer is the only check. So write each rule **with the incident that produced it**. A bare prohibition gets rationalized away by the next person under deadline; a prohibition with a cost attached does not.
