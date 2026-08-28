# Skills Overview

Skills extend Codex with domain-specific knowledge and workflows. Install them
project-wide under `.agents/skills/` or user-wide under `~/.codex/skills/`.

## Installed Skills

### Development & Automation

| Skill | Purpose | Trigger Phrases |
|-------|---------|-----------------|
| **codex-browser** | Route browser work to native Codex controls | "go to url", "take screenshot", "fill form" |
| **typescript** | TS performance, tsconfig, type errors | "optimize typescript", TS error codes |
| **bun-development** | Bun runtime, migration from Node | Working with Bun projects |
| **docker-expert** | Docker builds, security, orchestration | Dockerfile work, container issues |
| **mcp-builder** | Create MCP servers for external APIs | "build mcp server" |
| **bun-testing** | bun:test — mocks, snapshots, coverage, integration | "bun test", "mock.module", "test with bun" |
| **monorepo-expert** | Turborepo + Bun workspaces: pipeline, cache, filters | "turbo.json", "cache miss", "affected packages", "monorepo" |
| **wasm-development** | WebAssembly: Emscripten + wasm-pack, JS boundary, opt/debug | "build WASM", "Emscripten", "wasm-pack", "wasm-bindgen" |
| **protobuf-grpc** | Protobuf/gRPC: buf + protobuf-es, envelope encryption, compat | "proto files", "protobuf", "gRPC", "buf generate" |

### Planning & Design

| Skill | Purpose | Trigger Phrases |
|-------|---------|-----------------|
| **brainstorming** | Explore ideas before implementation | Any creative/feature work (auto-triggers) |
| **feature-spec** | PRDs, requirements, scope management | "write prd", "define requirements" |
| **create-plans** | Hierarchical Codex-executable project plans | "plan this project", "phase plan" |
| **codex-prd** | PRDs optimized for agent execution | "create a prd", "plan this feature" |

### Task Orchestration

| Skill | Purpose | Trigger Phrases |
|-------|---------|-----------------|
| **codex-beads** | Convert PRDs to executable beads; auto-detects the tracker CLI (cn/br/bd) | "create beads", "convert prd to beads" |

### Search & Growth

| Skill | Purpose | Trigger Phrases |
|-------|---------|-----------------|
| **codex-seo** | Route evidence-led SEO audits and installed specialists | "SEO audit", "improve search visibility" |
| **app-store-optimization** | Audit Apple/Google listings with Rust metadata and experiment checks | "ASO audit", "optimize app listing", "store metadata" |

### Engineering Quality

| Skill | Purpose | Trigger Phrases |
|-------|---------|-----------------|
| **rust-clean-architecture** | Layering enforced as a test (domain ← infra); visibility, splits | "which layer", "clean architecture", "split this module", "pub or pub(crate)" |
| **rust-quality** | Workspace clippy/unsafe/rustdoc lint gates + cargo sort | "set up clippy", "workspace lints", "deny unsafe", "cargo sort" |
| **integration-test** | Audit coverage, write un-hangable cross-boundary tests | "audit tests", "test coverage", "write integration tests" |
| **arch** | C4 architecture diagrams in MermaidJS | "draw architecture", "create a diagram", "document system structure" |
| **systematic-debugging** | Diagnose-before-fix workflow; two-tier triage → deep dive | "debug this", "why is this failing", "troubleshoot", "diagnose" |
| **bundle-analysis** | Bundle size: snapshot/diff/budget, tree-shaking, deps | "check bundle size", "analyze bundle", "size diff", "tree-shake" |
| **security-review** | CSP/crypto/injection/CORS review checklist | "security review", "check for vulnerabilities", "CSP compliance" |
| **perf-review** | App-level hot-path performance review (allocations, blocking, batching) | "review for performance", "hot path review", "performance audit" |
| **explain-module** | Structured module explanation: file:line, data-flow + dependency Mermaid | "how does X work?", "explain the Y module", "walk me through" |

### Backend & Deploy

| Skill | Purpose | Trigger Phrases |
|-------|---------|-----------------|
| **elixir-expert** | Phoenix/OTP: supervision trees, Rustler NIFs, Ecto, releases | "Phoenix", "GenServer", "supervisor", "Rustler NIF", "mix release" |
| **fastify-expert** | Fastify APIs: plugins, lifecycle hooks, JWT, `inject()` tests | "fastify", "route plugin", "preHandler hook", "fastify jwt" |
| **gcp-cloudrun** | Cloud Run deploy/rollback, Terraform, cold-start/OOM debug | "deploy to cloud run", "rollback deployment", "terraform cloud run" |

### Meta

| Skill | Purpose | Trigger Phrases |
|-------|---------|-----------------|
| **find-skills** | Discover & install new skills | "find a skill for X", "how do I do X" |
| **codex-skill-development** | Repository skill authoring with Codex gates | "create a repository skill" |
| **skill-autoresearch** | Improve skills through frozen evals and bounded Rust experiment loops | "autoresearch this skill", "optimize skill" |

## Skill Locations

```
~/.codex/skills/
├── typescript/                    # Direct install
├── bun-development/               # Direct install
├── docker-expert/                 # Direct install
├── codex-skill-development/       # Direct install
├── brainstorming -> ~/.agents/skills/brainstorming
├── codex-browser/                 # Direct install
├── feature-spec -> ~/.agents/skills/feature-spec
├── find-skills -> ~/.agents/skills/find-skills
├── mcp-builder -> ~/.agents/skills/mcp-builder
├── codex-beads -> ~/.agents/skills/codex-beads
├── codex-prd -> ~/.agents/skills/codex-prd
├── codex-seo -> ~/.agents/skills/codex-seo
├── app-store-optimization -> ~/.agents/skills/app-store-optimization
├── skill-autoresearch -> ~/.agents/skills/skill-autoresearch
├── rust-clean-architecture -> ~/.agents/skills/rust-clean-architecture
├── rust-quality -> ~/.agents/skills/rust-quality
├── integration-test -> ~/.agents/skills/integration-test
├── arch -> ~/.agents/skills/arch
├── systematic-debugging -> ~/.agents/skills/systematic-debugging
├── bun-testing -> ~/.agents/skills/bun-testing
├── monorepo-expert -> ~/.agents/skills/monorepo-expert
├── bundle-analysis -> ~/.agents/skills/bundle-analysis
├── security-review -> ~/.agents/skills/security-review
├── elixir-expert -> ~/.agents/skills/elixir-expert
├── fastify-expert -> ~/.agents/skills/fastify-expert
├── gcp-cloudrun -> ~/.agents/skills/gcp-cloudrun
├── explain-module -> ~/.agents/skills/explain-module
├── perf-review -> ~/.agents/skills/perf-review
├── wasm-development -> ~/.agents/skills/wasm-development
└── protobuf-grpc -> ~/.agents/skills/protobuf-grpc
```

## Installing Skills

```bash
# User-wide install (symlink from custom location)
ln -s /path/to/skill ~/.codex/skills/skill-name

# Project install
mkdir -p .agents/skills
cp -R /path/to/skill .agents/skills/skill-name
```
