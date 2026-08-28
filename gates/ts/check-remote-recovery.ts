#!/usr/bin/env bun
/**
 * A screen that could not establish its data must leave the reader something to
 * do about it.
 *
 * The primary mechanism is a TYPE, not this script: `RemoteUnreachable`
 * (`remote-state.ts`) carries its recovery action as a REQUIRED field, so a dead
 * end cannot be constructed. This gate is the backstop for the case the compiler
 * cannot see — a component that hand-rolls its own `error` flag and renders it
 * as a sentence with nothing beside it. That is the shape the reported bug had:
 * a shipped build told a user their courses would appear "once it can reach the
 * server again", and nothing re-checked.
 *
 * What it flags: in a component that reaches remote data, a template branch
 * keyed on a FAILURE (`{:catch}`, or a condition naming `error` / `failed` /
 * `failure` / `unreachable`) whose body offers the reader nothing — no
 * `<RemoteDeadEnd>`, no button, no click handler, no rendered snippet.
 *
 * What it deliberately does NOT do: judge whether an EMPTY state deserves a
 * button. "No courses are turned on — your admin chooses" is correct exactly
 * because it has none, and a gate that guessed here would push authors toward
 * offering actions the reader cannot take — the failure mode
 * `rules/ui-remote-states.md` cares about most. Emptiness is the semantic call
 * the type carries; this script only reads failure branches.
 *
 * There is no allowlist. Survey the offenders and fix them first, so that can
 * stay true.
 *
 * Usage:
 *   check-remote-recovery.ts <root> [root…]
 *   REMOTE_RECOVERY_ROOTS="apps/web/src,packages/ui/src" check-remote-recovery.ts
 *
 * WHY TypeScript and not Rust, which is the default for tooling here: this reads
 * Svelte templates, and a real parse of one belongs in the ecosystem that has
 * one. It also runs beside the frontend's other checks, under a runtime the repo
 * already installs.
 */

import { Glob } from "bun"

interface Violation {
  path: string
  line: number
  branch: string
}

const ROOTS = (
  process.argv.slice(2).length > 0
    ? process.argv.slice(2)
    : (process.env.REMOTE_RECOVERY_ROOTS ?? "").split(",")
)
  .map((r) => r.trim())
  .filter(Boolean)

if (ROOTS.length === 0) {
  console.error("check-remote-recovery: no source roots given.")
  console.error("  Pass them as arguments, or set REMOTE_RECOVERY_ROOTS=a,b.")
  console.error("  A gate with no roots would pass by reading nothing.")
  process.exit(2)
}

/** The component talks to something it does not control. */
const REMOTE_CALL =
  /(^|[^\w.])(invoke|executeWebOperation|fetch)\s*\(|\bports\.[A-Za-z]+[.(]|\bprops\.onload\b|\bonload\s*\(\)|\bawait\s+[A-Za-z_$][\w$]*(Api|Async)?\s*\(/

/** A branch on a failure — never on an empty. */
const FAILURE_CONDITION = /\b(error|errors|failed|failure|unreachable|loadError|lastError)\b/i
/**
 * Two shapes that carry the word and are not a failed read of ours:
 *
 *   - `columnKey === "failed"` / `testState.kind === "failure"` — a branch on the
 *     value of something we successfully fetched, e.g. one row's own status.
 *   - `!loadError && …` / `!feedback.startsWith("Error")` — the HAPPY arm,
 *     written as the negation. Its body is the content, not a message.
 */
const BRANCH_ON_A_FETCHED_VALUE = /===\s*["'](error|failed|failure)["']/i

/**
 * True when at least one operand of the condition asserts a failure. Operands
 * are split on `&&` / `||`, and a negated one asserts the opposite — its body is
 * the content, not a message about a failure.
 */
function assertsAFailure(condition: string): boolean {
  return condition
    .split(/&&|\|\|/)
    .map((operand) => operand.trim())
    .some(
      (operand) =>
        !operand.startsWith("!") && FAILURE_CONDITION.test(operand) && !BRANCH_ON_A_FETCHED_VALUE.test(operand)
    )
}

/** Something the reader can act on. */
const OFFERS_AN_ACTION = /<RemoteDeadEnd|<Button\b|<button\b|onclick=|<ErrorState\b|<FeatureUnavailable\b|\{@render\s/

const OPENERS = /\{#(if|each|await|key|snippet)\b/g
const CLOSERS = /\{\/(if|each|await|key|snippet)\}/g

function countMatches(line: string, pattern: RegExp): number {
  pattern.lastIndex = 0
  let n = 0
  while (pattern.exec(line) !== null) n += 1
  return n
}

/**
 * The lines belonging to a branch that starts at `start`: everything until the
 * next `{:else…}` or block close at the SAME nesting depth.
 */
function branchBody(lines: string[], start: number): string {
  const body: string[] = []
  let depth = 0
  for (let i = start; i < lines.length; i++) {
    const line = lines[i]
    if (i > start) {
      const atBranchDepth = depth === 0
      if (atBranchDepth && /^\s*\{:(else|then|catch)/.test(line)) break
      if (atBranchDepth && /^\s*\{\/(if|await|each|key)\}/.test(line)) break
    }
    body.push(line)
    depth += countMatches(line, OPENERS) - countMatches(line, CLOSERS)
    if (i > start && depth < 0) break
  }
  return body.join("\n")
}

const violations: Violation[] = []
const glob = new Glob("**/*.svelte")
let scanned = 0

for (const root of ROOTS) {
  let seenInRoot = 0
  for await (const relPath of glob.scan({ cwd: root, absolute: false })) {
    seenInRoot += 1
    scanned += 1
    const path = `${root}/${relPath}`
    const content = await Bun.file(path).text()
    if (!REMOTE_CALL.test(content)) continue

    const lines = content.split("\n")
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i]
      const isCatch = /^\s*\{:catch\b/.test(line)
      // Read the CONDITION, not the line: a branch whose body happens to print
      // the word "error" is not a branch on one.
      const condition = /^\s*\{(?:#if|:else if)\s+([\s\S]*?)\}/.exec(line)?.[1] ?? ""
      const isFailureIf = condition !== "" && assertsAFailure(condition)
      if (!isCatch && !isFailureIf) continue

      const body = branchBody(lines, i)
      if (OFFERS_AN_ACTION.test(body)) continue
      violations.push({ path, line: i + 1, branch: line.trim() })
    }
  }
  // A root that holds nothing is the silent failure this gate is most prone to:
  // a directory moved, and the check passes by reading zero files.
  if (seenInRoot === 0) {
    console.error(`✗ ${root} holds no .svelte files — this gate would pass by reading nothing.`)
    console.error("  A source root moved. Fix the roots passed to check-remote-recovery.ts.")
    process.exit(2)
  }
}

if (violations.length > 0) {
  console.error("✗ remote failure state(s) with no way out:\n")
  for (const v of violations) {
    console.error(`  ${v.path}:${v.line}`)
    console.error(`    ${v.branch}`)
  }
  console.error(
    "\nA reader who lands on one of these can only close the app.\n" +
      "  • Build the state with `remoteUnreachable({ message, retry })` and render it\n" +
      "    with <RemoteDeadEnd state={…} />. The retry RE-FETCHES — never\n" +
      "    location.reload(), which tears down a whole desktop app to fix one panel.\n" +
      "  • An authoritative empty is a different state: `remoteEmpty({ message, actor })`,\n" +
      "    which cannot carry a retry at all. This gate never reads empty branches.\n" +
      "  • Rule: rules/ui-remote-states.md\n"
  )
  process.exit(1)
}

console.log(`✓ every remote failure state offers a way out (${scanned} components across ${ROOTS.length} source roots)`)
