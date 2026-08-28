---
name: codex-browser
description: Route browser interaction to Codex's bundled in-app Browser or connected Chrome while preferring purpose-built connectors for semantic work. Use for navigation, visual inspection, forms, screenshots, logged-in browser state, and local web UI verification. Replaces legacy dev-browser workflows without a repository-local runtime.
---

# Codex Browser

Use Codex browser controls. Do not install or start the legacy `dev-browser`
Node server, extension, or Playwright package from this repository.

## Choose Surface

Honor an explicit browser choice.

- Use `$chrome:control-chrome` when the request names Chrome or depends on
  existing Chrome tabs, login state, or extensions.
- Use `$browser:control-in-app-browser` when the request names the in-app
  Browser or needs isolated navigation, interaction, screenshots, or local UI
  testing.
- When no browser is named, use the bundled browser skill available in the
  current session and let its selection rules choose the surface.
- Before semantic work on a linked resource, prefer an available connector,
  API, or CLI. Use browser control when UI interaction or visible state matters,
  or when no purpose-built capability can do the job.
- Use read-only web retrieval for public research that needs no interactive
  browser state.

Before controlling a selected browser, read its installed skill completely and
follow its setup, selection, authentication, and recovery rules. Those bundled
instructions own browser runtime details and may change independently from this
router.

## Operate

1. Select the least-privileged surface that preserves needed session state.
2. Inspect current visible state before interacting.
3. Make incremental actions and verify each consequential state change.
4. Never inspect cookies, password stores, profiles, or raw session storage.
5. Ask before actions with external consequences when user intent is not
   already explicit.
6. Report outcome and any remaining user action, such as signing in to the
   explicitly selected browser.

## Failure Boundary

If bundled browser runtime files or required browser tools are unavailable,
report that exact missing capability. Do not silently fall back to an unrelated
browser, standalone Playwright, or the incomplete legacy package.
