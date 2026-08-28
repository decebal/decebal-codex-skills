# Backward compatibility

Protobuf's whole value is that an old decoder can read a new message and a new
decoder can read an old one. That holds only if you respect the wire contract. A
decoder keys off **field numbers**, not names — so the rules are about numbers and
types, not identifiers.

## The rules

- **Field numbers are immutable — never reuse, never renumber.** Reusing a retired
  number makes a new field silently decode old data of a different type. Renumbering
  an existing field breaks every peer that hasn't updated.
- **Adding fields is safe.** Give each a fresh, never-before-used number. Absent
  fields decode to their default (0, `""`, empty, `false`), so old peers ignore
  new fields and new peers see defaults from old peers.
- **Renaming a field is safe on the binary wire** (the number is unchanged) but
  **breaks JSON**, which keys off the name. If you use `toJson`/`fromJson` across
  versions, treat names as part of the contract too.
- **Do not change a field's type.** A few changes are wire-compatible (e.g.
  `int32`/`int64`/`uint32`/`uint64`/`bool`/`enum` share the varint wire type;
  `string` and `bytes` interoperate when the bytes are valid UTF-8), but `sint*`
  (zigzag) is **not** compatible with `int*`, and 32-bit fixed types are not
  compatible with 64-bit ones. The safe rule: add a new field instead of changing
  a type.
- **`reserved` removed fields' numbers AND names** so nobody revives them:

  ```proto
  message User {
    reserved 2, 5, 9 to 11;        // retired field numbers
    reserved "email", "phone";     // retired field names
    string id = 1;
    string display_name = 3;
  }
  ```

- **Enum zero value is the default and must exist.** proto3 requires the first
  enum value to be `0`; name it `*_UNSPECIFIED` so "unset" is distinguishable and
  never collides with a meaningful member. Reserve removed enum values too.

  ```proto
  enum Status {
    STATUS_UNSPECIFIED = 0;        // default; do not repurpose
    STATUS_ACTIVE      = 1;
    STATUS_ARCHIVED    = 2;
    reserved 3;                    // a retired status
  }
  ```

## Well-known types: Timestamp / Duration vs native `Date`

Use `google.protobuf.Timestamp` and `google.protobuf.Duration` rather than
inventing a `seconds`/`nanos` pair or an int64-of-millis — peers, tooling, and
JSON mapping all understand them.

```proto
import "google/protobuf/timestamp.proto";
import "google/protobuf/duration.proto";

message Session {
  google.protobuf.Timestamp started_at = 1;
  google.protobuf.Duration  ttl        = 2;
}
```

protobuf-es v2 provides converters in `@bufbuild/protobuf/wkt` — a `Timestamp` is
not a JS `Date`, so convert at the boundary:

```ts
import { create } from "@bufbuild/protobuf";
import { timestampFromDate, timestampDate, timestampNow } from "@bufbuild/protobuf/wkt";
import { SessionSchema } from "./gen/session_pb";

const s = create(SessionSchema, { startedAt: timestampNow() });
const jsDate: Date = timestampDate(s.startedAt!);          // Timestamp -> Date
const fromNative = timestampFromDate(new Date());          // Date -> Timestamp
```

## Enforce it in CI, don't rely on review

A renumber or a type change is easy to miss in a diff. Make `buf breaking` the
gate — it compares the current schema against the trunk and fails on any wire- or
JSON-breaking change:

```bash
bunx @bufbuild/buf breaking --against '.git#branch=main'
```

Wire it as a required check ([../../../rules/testing-gates.md](../../../rules/testing-gates.md)),
and read the tool's own output rather than trusting a green badge
([../../../rules/evidence-discipline.md](../../../rules/evidence-discipline.md)).
Pick the `breaking.use` category in `buf.yaml` to match your compatibility
promise: `FILE` is strictest (also catches source-level breaks like field
renames); `WIRE` / `WIRE_JSON` allow renames that keep the wire/JSON stable.
