# {SDK Name}

For SDKs and embeddable libraries loaded inside third-party host applications.
Use this instead of `fullstack-app.md` when your build output runs on someone
else's page: bundle size, CSP, and side-effect isolation are hard constraints,
and the public API is a contract.

## Rules

Render this overlay with `render-agent-docs`; embed these portable fragments in
order: `git-discipline`, `evidence-discipline`, `definition-of-done`, `comments`,
`testing-gates`, `dependency-hygiene`, `error-channels`. Codex does not expand
Markdown `@` imports.

## Project Overview
{SDK name, what it does, how integrators embed it.}

## Integration Model
- **Load path**: {`<script>` tag / npm package / CDN / all three}
- **Entry point**: {Web Component `<my-widget>` / global `window.MySdk` / ESM export}
- **Framework compatibility**: framework-agnostic; document any React/Vue/Svelte wrappers
- **Isolation**: {Shadow DOM / iframe / none} — say how host styles and the SDK are kept apart

## Build & Bundle
```bash
{bun run build}          # produce dist/ (ESM + UMD, minified)
{bun run size}           # print per-entry gzip sizes
```
- **Bundle-size budgets** (gzip, enforced in CI — a regression fails the build):
  | Entry point | Budget |
  |-------------|--------|
  | main        | {NN KB} |
  | {lazy chunk}| {NN KB} |
- **Tree-shaking**: `sideEffects: false` in package.json; no top-level side effects on import
- **Obfuscation** (commercial SDKs): {JScrambler/…} runs in CI, not locally; creds are not in the repo

## Public API Contract
- Exported types/interfaces are the contract — additive changes only within a major
- **Semver**: breaking change ⇒ major; new API ⇒ minor; fix ⇒ patch
- **Deprecation**: mark `@deprecated` with the replacement + the version it's removed in; never remove without a major
- Keep `.d.ts` output in sync with the runtime surface (generate it, don't hand-write)

## Security Constraints
- **CSP**: no `eval`, no `new Function`, no inline `style=`/`<style>` injection without a nonce; enforce it in the build (e.g. a CSP plugin), not by review alone
- **Sandboxing**: {Shadow DOM for style isolation / iframe for full isolation}
- **The SDK must NEVER**: read host cookies/localStorage it doesn't own, mutate host DOM outside its own root, register global listeners it doesn't remove, or leak globals beyond its single namespace
- Third-party dependencies are attack surface on the host page — justify each; prefer zero runtime deps

## Testing
- **Unit**: {Vitest/Bun} — pure logic
- **Integration**: mount the SDK in a real DOM (jsdom/happy-dom) and drive the public API
- **Cross-browser matrix**: {Chromium, Firefox, WebKit} via {Playwright}
- **Bundle-size regression**: compare gzip sizes against the committed baseline; fail on growth over budget
- **CSP smoke test**: load the built bundle under a strict CSP and assert no violations

## Release Process
```bash
{bun run release}        # version bump + changelog + tag
```
1. Bump version (semver) — a breaking API change is a major, no exceptions
2. Generate the changelog from conventional commits
3. Publish to npm and/or deploy to the CDN (immutable, versioned paths)
4. Tag only after CI is green — never tag a red build

## DO NOT
- Ship a bundle over budget "temporarily"
- Add a runtime dependency without a size + security justification
- Break the public API outside a major version
- Assume the host page's CSP, framework, or global environment
- Pollute global scope or leave listeners/timers running after teardown
