---
title: State Debugging
tags: frontend-state, stale-closure, re-render, database, cache, last-writer-wins
---

# State Debugging

State bugs share one shape: the value you READ is not the value you expect, because
you are reading the wrong version, the wrong copy, or the wrong store. Find out which
before touching logic.

## Frontend state

- **Stale closure.** A callback captured an old value at definition time and keeps
  using it — the classic "click handler logs the count from three renders ago."
  Symptom: the UI shows the new value but the handler acts on the old one. Fix by
  reading from a ref, a functional updater, or correcting the dependency list so the
  closure is recreated when the value changes.
- **Unexpected / missing re-render.** Either the component re-renders on every keypress
  (a new object/array/function identity passed each render) or it never updates (you
  mutated state in place instead of replacing it, so identity did not change and the
  framework saw nothing). Log the identity of the suspect value across renders — same
  reference when it should differ, or differing when it should be stable, names the
  bug.
- Instrument first: log the value AND its source at the render site and at the read
  site. A value that is right at one and wrong at the other localizes the bug to the
  hop between them.

## Database state

- **Read the store that HOLDS the answer.** Systems commonly have more than one — a
  local/per-machine store and a hosted/per-tenant one — and they are not
  interchangeable. Querying the wrong one returns empty, which reads exactly like "no
  such data." Say which store you queried.
- **Fold the history to get current state.** In an event-sourced or append-only store,
  reading one row gives you a moment, not the value. Reduce the full event stream,
  last-writer-wins by timestamp, to get the current truth.
- **A cheap health probe first.** Check row counts and the newest timestamp before
  concluding "no records." Old dates or a tiny count mean your tool is pointed at a
  dev/archived copy, not the live store.
- **File size lies for WAL-backed stores.** A 0-byte database file with data in
  adjacent segment/WAL files is normal — read the segments.

See [../../../rules/evidence-discipline.md](../../../rules/evidence-discipline.md) — read
runtime and store state, never guess it.

## Cache state

Every cache bug is one of two:
- **Staleness** — the cache returns a value that the source of truth has since
  changed. The read is "correct" for a version that no longer exists. Check the TTL and
  the invalidation path: is anything evicting on write, or only on expiry?
- **Key mismatch** — the write and the read compute different keys (a trailing slash, a
  serialized-object key whose field order varies, a missing tenant prefix), so the read
  is a permanent miss that looks like the cache "not working." Log the exact key on both
  the write and the read path and diff them byte-for-byte.

Discriminating test: bypass the cache (read the source directly). Correct without the
cache and wrong with it → staleness or key mismatch. Wrong both ways → the bug is
upstream in the source, not the cache.
