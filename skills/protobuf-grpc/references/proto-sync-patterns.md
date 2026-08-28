# Cross-repo `.proto` sync

When more than one service or app shares message types, the `.proto` files are a
shared contract. Copy-pasting them by hand guarantees drift: two repos silently
diverge on a field number and the wire breaks in production. Keep one source of
truth and sync from it with provenance.

## Layout: one owner, many consumers

```
proto-repo/                 # the single owner of the .proto files
  proto/
    chat/v1/chat.proto
    envelope/v1/envelope.proto
  buf.yaml
  VERSION                   # e.g. 1.4.0 — bumped on every schema change

service-a/ (consumer)
  proto/                    # synced copy — NEVER hand-edited
  proto/.proto-version      # provenance: which owner version this copy came from
  src/gen/                  # generated + committed
```

Options for the owner:

- **A dedicated git repo** synced via a script (below), or a git submodule /
  subtree if the team prefers git-native pinning.
- **The Buf Schema Registry (BSR)** — publish the module, and consumers declare it
  under `deps:` in `buf.yaml` and pull with `bunx @bufbuild/buf dep update`. This
  replaces the sync script entirely and is the buf-native path.

## A sync script with provenance

The point of the `VERSION` / `.proto-version` files: an absence or mismatch is
diagnosable. If generated code looks wrong, you can prove which schema version
produced it instead of guessing
([../../../rules/evidence-discipline.md](../../../rules/evidence-discipline.md)).

```bash
#!/usr/bin/env bash
set -euo pipefail
# sync-protos.sh — run in the CONSUMER repo
OWNER_REPO="git@github.com:org/proto-repo.git"
REF="${1:-main}"                 # tag or branch to pin
TMP="$(mktemp -d)"

git clone --depth 1 --branch "$REF" "$OWNER_REPO" "$TMP"

VERSION="$(cat "$TMP/VERSION")"
rm -rf proto
cp -R "$TMP/proto" proto
printf '%s\n' "$VERSION" > proto/.proto-version   # provenance stamp

bunx @bufbuild/buf lint
bunx @bufbuild/buf generate                        # regenerate src/gen from the fresh protos
rm -rf "$TMP"

echo "synced protos @ $VERSION (ref $REF); review git diff before committing"
```

## Rules

- **Generated code is committed downstream.** Consumers build without the proto
  owner or the buf toolchain present, and the wire types are visible in review.
- **The synced `proto/` copy is read-only** in the consumer — edits go to the
  owner, then flow down. Enforce with a CI check that the copy matches the pinned
  `VERSION`, or a CODEOWNERS rule.
- **Bump `VERSION` on every schema change** in the owner, and run `buf breaking`
  there so a breaking edit is caught before any consumer syncs it (see
  [backward-compatibility.md](backward-compatibility.md)).
- **Pin the ref** (a tag, not a floating branch) so a sync is reproducible.
