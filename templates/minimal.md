# {Project Name}

## Rules

Render this overlay with `render-agent-docs`; embed these portable fragments in
order: `git-discipline`, `evidence-discipline`, `comments`, `token-efficiency`.
Codex does not expand Markdown `@` imports.

## Stack
- Runtime: Bun
- Language: TypeScript

## Commands
```bash
bun install          # Install dependencies
bun run dev          # Start dev server
bun run build        # Production build
bun run test         # Run tests
bun run lint         # Lint with Biome
```

## Conventions
- Package manager: Bun only (never npm/yarn)
- Linting: Biome (single quotes JS, double quotes JSX, 100 char width)
- Always run `bun run lint` before committing

## DO NOT
- Mix package managers
- Commit without running lint + tests
- Push to `main` — feature branch and PR, always
