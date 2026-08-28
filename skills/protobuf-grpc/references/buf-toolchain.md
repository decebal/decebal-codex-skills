# buf toolchain

`buf` is the modern Protobuf driver: one config for lint, generate, and
breaking-change detection, with remote or local plugins. The CLI ships as a Go
binary, but the npm wrapper `@bufbuild/buf` lets you drive it with `bunx` and pin
its version in `package.json` alongside the plugins.

```bash
bun add -d @bufbuild/buf @bufbuild/protoc-gen-es @bufbuild/protobuf
# for gRPC-Web clients as well:
bun add @connectrpc/connect @connectrpc/connect-web
```

## `buf.yaml` — the module config

Lives at the repo root (or the proto root). v2 format:

```yaml
version: v2
modules:
  - path: proto
lint:
  use:
    - STANDARD          # STANDARD | BASIC | MINIMAL | COMMENTS | UNARY_RPC
breaking:
  use:
    - FILE              # FILE (strictest) | PACKAGE | WIRE_JSON | WIRE
```

If your protos import Buf Schema Registry modules (e.g. well-known types beyond
the built-ins), declare them under `deps:` and run `bunx @bufbuild/buf dep update`
to write `buf.lock`.

## `buf.gen.yaml` — the generation config

Controls which plugins run and where output lands. Remote plugin (no local
install, runs on the BSR):

```yaml
version: v2
clean: true                       # wipe out dirs before writing
plugins:
  - remote: buf.build/bufbuild/es
    out: src/gen
    opt:
      - target=ts                 # target=ts | js+dts | js | dts
```

Local plugin (uses the `protoc-gen-es` binary from `@bufbuild/protoc-gen-es`,
which keeps codegen reproducible and offline):

```yaml
version: v2
clean: true
plugins:
  - local: protoc-gen-es
    out: src/gen
    opt:
      - target=ts
```

`protoc-gen-es` (protobuf-es v2) also generates service descriptors, so Connect-ES
needs no separate plugin — `createClient(MyService, transport)` consumes the
service exported from the same `_pb.ts` file.

## Commands

```bash
# Lint against the rules in buf.yaml
bunx @bufbuild/buf lint

# Auto-format .proto files in place
bunx @bufbuild/buf format -w

# Generate code per buf.gen.yaml (run from the dir holding it)
bunx @bufbuild/buf generate

# Build a descriptor set (parse + validate everything, no output)
bunx @bufbuild/buf build

# Breaking-change check against the trunk — this is the CI gate
bunx @bufbuild/buf breaking --against '.git#branch=main'
# ...or against a checked-in descriptor image:
bunx @bufbuild/buf breaking --against 'image.binpb'
```

Point the client at generated code and a gRPC-Web transport:

```ts
import { createClient } from "@connectrpc/connect";
import { createGrpcWebTransport } from "@connectrpc/connect-web";
import { ChatService } from "./gen/chat_pb";        // service schema from protoc-gen-es

const transport = createGrpcWebTransport({ baseUrl: "https://api.example.com" });
const client = createClient(ChatService, transport);
```

## Commit the output

Check the generated `src/gen/**` into git. Downstream consumers and CI then build
without the buf toolchain, and diffs to the wire types are visible in review.
Never assert what a generated symbol is named — open the `_pb.ts` file
([../../../rules/evidence-discipline.md](../../../rules/evidence-discipline.md)).
Keep the plugin/dep set minimal
([../../../rules/dependency-hygiene.md](../../../rules/dependency-hygiene.md)).
