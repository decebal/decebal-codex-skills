# {Project Name}

## Rules

Render this overlay with `render-agent-docs`; embed these portable fragments in
order: `git-discipline`, `evidence-discipline`, `agent-parallelism`, `timeouts`,
`definition-of-done`, `comments`, `debugging-discipline`, `token-efficiency`,
`testing-gates`, `layer-boundaries`, `dependency-hygiene`, `error-channels`,
`event-streams`, `ui-remote-states`. Codex does not expand Markdown `@` imports.

## Stack
- Desktop framework: Tauri 2 ({macOS/Windows})
- Backend: Rust (clean architecture, 4 layers), IPC via **AllFrame** typed router
- Frontend: Svelte 5 (runes) + TypeScript, domain architecture
- Persistence: **AllSource** event sourcing — events are the source of truth;
  in-memory projections are rebuilt from the event log on startup (no SQLite/ORM)
- Styling: Tailwind CSS

## Commands
```bash
bun run dev                      # Start all (frontend + backend)
bun run quality                  # Lint + format + typecheck + test
bun test                         # Frontend tests (Bun)
cargo test -- --test-threads=1   # Rust tests (sequential — shared event-store state)
```

## Project Structure
```
src-tauri/
  src/
    presentation/    # Tauri IPC handlers / router
    application/     # Services (orchestration)
    infrastructure/  # Persistence + external I/O
    domain/          # Entities, pure logic
  Cargo.toml
src/
  lib/
    domains/{name}/  # components, state, services, index.ts barrel
    shared/          # cross-domain UI + services
    infrastructure/  # IPC adapters
  routes/
```

## Architecture
- Frontend ↔ Backend via **AllFrame** typed handlers, not raw `#[tauri::command]`:
  `register_typed`, `register_result_with_args`, `register_with_state`. Handlers
  are thin delegates — business logic lives in `application/services/`
- **AllSource** event sourcing: write events, derive state; projections are
  in-memory and rebuilt from the log on startup
- Dependency direction: presentation → application → infrastructure ← domain,
  **enforced by an architecture test suite**, not by convention
- Event streams: Activity (telemetry, never toasts) vs Alerts (always toasts)
- Frontend is domain-organized (`domains/{name}/` with `components/`, `state/`,
  `services/`), not organized by component type
- Services never import infrastructure directly — go through a facade

## Conventions
- **Rust tests run under `cargo nextest`** — process-per-test, so `--test-threads=1`
  is not needed. Serialize only what is machine-global (the OS keychain) in one
  named group with `max-threads = 1`
- **Invoke args are camelCase** — `invoke("cmd", { workflowId })`, not
  `{ workflow_id }`. AllFrame does NOT auto-convert; a name mismatch is a
  silently dead command. Gate it
- **Svelte 5 runes** — `$state`, `$derived`, `$effect`; NOT `writable`/`derived`
  stores. Domain state is a rune-based module under `domains/{name}/state/`
- Frontend: typed IPC wrappers, one per domain, checked against a command map
- Errors: `notifyError(e, source, phase)` for user-facing failures;
  `logError(e, source, phase)` for dev-only. Rust `thiserror` types map to
  human-readable frontend messages
- Clipboard: never `navigator.clipboard` — it silently fails in the desktop
  webview. Use a wrapper

## DO NOT
- Use `unwrap()` in production Rust code
- Access persistence directly from IPC handlers (route through services)
- Use `requestAnimationFrame` positioning for popovers — it produces phantom
  clicks in the desktop webview
- Render a raw payload to a user
- Run two cargo builds at once
