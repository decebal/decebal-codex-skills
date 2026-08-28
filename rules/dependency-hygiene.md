# Dependency hygiene

The dependency graph is a build-time and stability cost. Keep it small and
single-versioned.

## Before adding a package

1. **Do we already have it?** Grep the manifests / run the tree command. Reuse an
   existing dep before adding a new one; prefer the standard library for small
   needs.
2. **Check the blast radius.** `cargo tree -i <crate>` / `npm ls <pkg>` — a package
   that drags in 40 transitive deps is rarely worth a one-liner. Look for a lighter
   alternative.
3. **Don't duplicate a version.** If the package (or a shared transitive) already
   resolves to a version in the tree, match it — a second major version compiles
   separately AND risks behaviour skew.
4. **Trim features.** Add with `default-features = false` and enable only what you
   use, especially for the heavy ones.

## Don't leave dead weight

- **No unused deps.** If you stop using a package, remove it from the manifest.
  `cargo machete --with-metadata` finds them — verify, it false-positives on
  build-deps, dev-deps, and macro/cfg-only uses; declare those as ignored in
  package metadata.
- **No speculative deps.** Don't declare a package a future phase will need. A
  Phase-1 stub declaring its Phase-2 deps bloats every build in between.

## Metric

Record a baseline and watch for regressions:

```bash
# unique crates
cargo tree -e no-dev --prefix none | grep -E '^[a-z0-9_-]+ v' | sed -E 's/ .*//' | sort -u | wc -l
# duplicated versions
cargo tree --duplicates -e no-dev | grep -E '^[a-z0-9_-]+ v' | sed -E 's/ .*//' | sort -u | wc -l
```

A PR that meaningfully grows either should say why. Run the advisory/license/
duplicate policy check (`cargo deny check`) before landing dependency changes.
