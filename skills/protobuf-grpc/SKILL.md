---
name: protobuf-grpc
description: "Build Protocol Buffers and gRPC or gRPC-Web integrations. Use for proto schemas, code generation, Buf workflows, binary and JSON serialization, encrypted envelopes over WebSockets, TypeScript browser clients, and wire-compatible schema evolution."
---

# Protocol Buffers + gRPC(-Web)

Schema-first binary messaging: define types once in `.proto`, generate typed
code, exchange compact binary frames. This skill is opinionated for the browser
(strict CSP, ESM, small bundles) but the workflow is the same everywhere.

## Pick the codegen BEFORE writing a `.proto`

The wrong runtime is expensive to unwind after the schema and call sites exist.

- **Toolchain: prefer `buf` over `protoc`.** buf is the modern driver — one YAML
  config, built-in lint and breaking-change detection, remote plugins, no
  `protoc` install or hand-managed `--*_out` flags. Treat raw `protoc` as legacy.
- **TS runtime: prefer `protobuf-es` (`@bufbuild/protobuf`) over `google-protobuf`.**
  `google-protobuf` uses `new Function()` in its reflection/serialization paths,
  which a strict CSP without `unsafe-eval` blocks — the code dies on load on a
  hardened page. `protobuf-es` is generated, ESM, tree-shakeable, and CSP-safe
  (no `eval`/`Function`). This is a security decision as much as a DX one — see
  the CSP angle in [../security-review/SKILL.md](../security-review/SKILL.md).
- **gRPC-Web: use Connect-ES** (`@connectrpc/connect` + `@connectrpc/connect-web`),
  which builds on `protobuf-es`. Browsers cannot speak raw gRPC (HTTP/2 trailers);
  gRPC-Web is the wire format a browser can reach.

Full matrix with runtime/ESM/CSP/gRPC-Web columns:
[references/codegen-comparison.md](references/codegen-comparison.md).

## The core loop

```bash
# 1. write proto/envelope.proto  (schema is the source of truth)
# 2. lint it
bunx @bufbuild/buf lint
# 3. generate typed code (config: buf.gen.yaml; module: buf.yaml)
bunx @bufbuild/buf generate
# 4. import the generated code from your app
```

**Commit the generated `_pb.ts` files.** Downstream builds, CI, and anyone
without the buf toolchain then compile reproducibly without running codegen — and
a reviewer can see exactly what the wire types are. Config and commands:
[references/buf-toolchain.md](references/buf-toolchain.md).

## Serialization: binary on the wire, JSON only for humans

protobuf-es v2 is schema-function based — you pass the generated `*Schema` to
free functions, not methods on an instance:

```ts
import { create, toBinary, fromBinary, toJson } from "@bufbuild/protobuf";
import { EnvelopeSchema, type Envelope } from "./gen/envelope_pb";

const msg = create(EnvelopeSchema, { sequence: 1n, payloadType: "Ping" });

const bytes = toBinary(EnvelopeSchema, msg);        // Uint8Array — THE wire format
const back  = fromBinary(EnvelopeSchema, bytes);    // Envelope

const debug = toJson(EnvelopeSchema, msg);          // logs/tests ONLY — larger, lossy for some types
// fromJson(EnvelopeSchema, debug) parses it back
```

Send binary over WebSockets as **binary frames**, never text:

```ts
ws.binaryType = "arraybuffer";
ws.send(toBinary(EnvelopeSchema, msg));                       // binary frame
ws.onmessage = (e) => fromBinary(EnvelopeSchema, new Uint8Array(e.data));
```

## The Envelope pattern

Wrap every message in one typed `Envelope` so the transport carries a uniform
shape: a **sequence number** for request/response correlation, and encryption
applied to the envelope's payload — AES-256-GCM with a fresh random 96-bit IV per
message (**never reuse an (IV, key) pair**). Full proto + crypto:
[references/envelope-encryption.md](references/envelope-encryption.md). The IV
rule and WebCrypto specifics live in
[../security-review/SKILL.md](../security-review/SKILL.md) (encryption-patterns).

## Backward compatibility is a wire contract

- **Field numbers are immutable.** Never reuse or renumber a field number — a
  decoder keys off the number, not the name. Renaming a field is safe on the
  binary wire (not in JSON).
- **Adding fields is safe**; give each a new, never-before-used number.
- **`reserved`** the numbers and names of removed fields so no one revives them.
- **Enforce it in CI** with `buf breaking` against the trunk — do not rely on
  review to catch a renumber. Rules and well-known types (Timestamp/Duration vs
  native `Date`): [references/backward-compatibility.md](references/backward-compatibility.md).

## Cross-repo `.proto` sync

When several services share schemas, keep the `.proto` in one place and sync with
provenance (a `VERSION` file), committing generated code downstream — never
hand-copy. Patterns: [references/proto-sync-patterns.md](references/proto-sync-patterns.md).

## References

- [references/buf-toolchain.md](references/buf-toolchain.md) — real `buf.yaml` + `buf.gen.yaml`, lint/generate/breaking/format commands.
- [references/codegen-comparison.md](references/codegen-comparison.md) — protobuf-es vs google-protobuf vs ts-proto vs protoc-gen-grpc-web.
- [references/proto-sync-patterns.md](references/proto-sync-patterns.md) — dedicated proto package, VERSION provenance, downstream commit.
- [references/envelope-encryption.md](references/envelope-encryption.md) — Envelope proto, AES-256-GCM at the envelope, sequence correlation, binary WS.
- [references/backward-compatibility.md](references/backward-compatibility.md) — field-number rules, `reserved`, enum zero value, well-known types, `buf breaking` in CI.

Adding a proto dependency? Keep the graph lean —
[../../rules/dependency-hygiene.md](../../rules/dependency-hygiene.md). Never
assert a generated symbol or wire value from memory; read the generated file —
[../../rules/evidence-discipline.md](../../rules/evidence-discipline.md).
