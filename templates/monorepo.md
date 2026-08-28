# {Project Name}

## Rules

Render this overlay with `render-agent-docs`; embed these portable fragments in
order: `git-discipline`, `evidence-discipline`, `agent-parallelism`, `timeouts`,
`definition-of-done`, `comments`, `token-efficiency`, `testing-gates`,
`layer-boundaries`, `dependency-hygiene`. Codex does not expand Markdown `@`
imports.

## Overview
Monorepo managed with {Turborepo/Nx/Taskfile}.

## Structure
```
apps/
  api/              # Backend API ({Elysia/Express/Actix})
  web/              # Frontend ({Next.js/Svelte/React})
packages/
  shared/           # Shared types and utilities
  ui/               # Shared UI components
  config/           # Shared configuration
```

## Package Manager
Bun only. NEVER use npm or yarn.

## Commands

### Global (from root)
```bash
task dev             # Start all services
task build           # Build everything
task test            # Run all tests
task lint            # Lint everything
task clean           # Clean build artifacts
```

### Per-app
```bash
task dev:api         # Start API only
task dev:web         # Start frontend only
task test:api        # Test API only
task test:web        # Test frontend only
```

### Docker
```bash
task docker:up       # Start Docker stack
task docker:down     # Stop Docker stack
task docker:logs     # View logs
```

## Architecture
{Describe key architectural decisions, data flow, service communication}

## Service Ports
Fill this in the moment there are 3+ services — running them locally without a
port map is guesswork.

| Service | Port | Protocol | URL |
|---------|------|----------|-----|
| web / plugin | 3000 | http | http://localhost:3000 |
| gateway | 8080 | http | http://localhost:8080 |
| api | 8081 | http | http://localhost:8081 |
| ws | 8082 | ws | ws://localhost:8082 |

## Authentication Flow
How services trust each other (verify a signature locally; don't round-trip to
an auth service on every request):
1. Client → Auth service: request a token
2. Auth service → Client: issue a signed JWT
3. Client → API service: request with the JWT in the `Authorization` header
4. API service: verify the JWT signature with the shared public key — no call back to auth

## Environment Setup
### Key generation
```bash
{task keys:generate}   # generate the JWT signing keypair / shared secrets into .env files
```
### Variable distribution matrix
Which `.env` var belongs to which service — a var in the wrong `.env` is a
silent misconfiguration.

| Variable | plugin | gateway | api | ws |
|----------|:------:|:-------:|:---:|:--:|
| `JWT_PUBLIC_KEY`  |   |   | ✅ | ✅ |
| `JWT_PRIVATE_KEY` |   | ✅ |    |    |
| `DATABASE_URL`    |   |   | ✅ |    |
| `PUBLIC_API_URL`  | ✅ |   |    |    |

## SSL / local HTTPS
Frontends that need secure context (WebAuthn, camera, service workers) need
local HTTPS:
```bash
{task certs:local}     # mkcert-based localhost cert; trust it once
```

## Infrastructure
### Deployment targets
| Service | Target |
|---------|--------|
| api / gateway / ws | {Cloud Run / ECS / Fly} |
| web / plugin | {CDN / static host} |

### CI/CD pipeline map
Which pipeline builds which service — path filters keep an api change from
rebuilding the frontend:
| Pipeline | Triggers on | Builds |
|----------|-------------|--------|
| api-ci | `apps/api/**`, `packages/shared/**` | api |
| web-ci | `apps/web/**`, `packages/ui/**` | web |

## Cross-service testing
- Integration tests that span services run against the Docker stack (`task docker:up`), not mocks
- Each such test owns its data setup + teardown; never depend on another test's leftovers
- Contract tests where two services share a schema — a schema change must fail both halves' gate

## MCP servers
Runtime debug tooling (log server, DB inspector) is configured in
`.codex/config.toml` per trusted project — see `configs/` and `docs/mcp-servers.md`.

## Gates

Every gate runs under `timeout -k 15 300`. A sub-project that is NOT a workspace
member needs its own dependency install, or its gate fails for a reason that has
nothing to do with the diff.

```bash
gates/sh/check-branch-not-merged.sh              # dead-branch guard, runs first
staged-scope --range "$BASE"                     # run only the gates this diff touches
```

## Conventions
- Linting: Biome (single quotes JS, double quotes JSX, 100 char width)
- Tests: {Vitest/Playwright} — always run before committing
- Commits: conventional commits (`feat:`, `fix:`, `chore:`)
- Branches: feature branches off `main`
- A package with test files but no `test` script runs in NO gate — check it

## DO NOT
- Mix package managers
- Import directly between apps (use `packages/`)
- Run two builds at once — one build on the machine at a time
- Commit without running lint + tests
