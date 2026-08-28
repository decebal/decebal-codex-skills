---
name: elixir-expert
description: "Build and operate Phoenix and OTP services in Elixir. Use for supervision trees, GenServer or Task design, umbrella projects, Ecto, LiveView, Rustler NIFs, Mix releases, Docker, Credo, Dialyzer, BEAM stalls, bottlenecks, or mailbox growth."
---

# Elixir / Phoenix / OTP Expert

Actionable guidance for a Phoenix + OTP service (e.g. a Query Service and an MCP
server fronting a Rust core via Rustler). Start here; the two references carry the
depth.

- [references/otp-patterns.md](references/otp-patterns.md) — GenServer skeleton, supervision trees, Registry/DynamicSupervisor, back-pressure, the "don't build a bottleneck" fix.
- [references/ecto-and-testing.md](references/ecto-and-testing.md) — schemas/migrations/changesets, `Repo` + `Ecto.Multi`, ExUnit `async`, Mox, SQL Sandbox, test containers.

## Project shape: single app vs umbrella

Default to a **single Phoenix app**. Reach for an **umbrella** (`mix new my_stack
--umbrella`, apps under `apps/*`) only when you have genuinely separate deployables
or independent release units that still share a repo — e.g. `apps/query_service`,
`apps/mcp_server`, `apps/core` (the Rustler NIF wrapper). An umbrella gives each app
its own `mix.exs`, supervision tree, and test suite; it does **not** give you a hard
compile-time boundary — `apps/*` can still call each other's modules. Enforce
direction with the layer rule below, not with the umbrella alone.

Map the layers ([../../rules/layer-boundaries.md](../../rules/layer-boundaries.md)) onto Phoenix:

| Layer | Phoenix home |
|---|---|
| presentation | `MyAppWeb` — controllers, channels, LiveViews, MCP tool handlers |
| application | context modules (`MyApp.Accounts`, `MyApp.Query`) — orchestration |
| infrastructure | `Repo`, HTTP clients, the Rustler NIF module, external adapters |
| domain | Ecto schemas + pure functions/structs, no I/O |

Direction is downward only. A context calls the Repo; a schema imports nothing
upward. See [../../rules/layer-boundaries.md](../../rules/layer-boundaries.md).

## Application lifecycle

`lib/my_app/application.ex` uses `Application` and returns the top supervision tree:

```elixir
defmodule MyApp.Application do
  use Application

  @impl true
  def start(_type, _args) do
    children = [
      MyApp.Repo,
      {Phoenix.PubSub, name: MyApp.PubSub},
      {Registry, keys: :unique, name: MyApp.SessionRegistry},
      {DynamicSupervisor, name: MyApp.SessionSup, strategy: :one_for_one},
      MyAppWeb.Endpoint
      # NIF crate loads on module first-use; no child needed for Rustler.
    ]

    Supervisor.start_link(children, strategy: :one_for_one, name: MyApp.Supervisor)
  end

  @impl true
  def start_phase(_phase, _type, _args), do: :ok
end
```

Child **order matters**: the Repo must start before anything that queries it; the
Endpoint goes last so it only accepts traffic once dependencies are up.

## OTP: pick the right abstraction

| Need | Use | Why |
|---|---|---|
| Long-lived mutable state, serialized access, must survive crashes | `GenServer` | full callback set, supervised, back-pressure via `call` |
| Fire-and-forget or await one concurrent computation | `Task` / `Task.Supervisor` | no state, no callbacks; `Task.async_stream` for bounded parallel maps |
| Trivial shared state, no logic | `Agent` | thin wrapper over a GenServer holding one value |
| No state at all | a plain module | don't spawn a process to hold nothing |

**A process should NOT hold state when the state is derivable, immutable config, or
better kept in ETS.** Spawning a GenServer to memoize a value every caller could
compute just adds a serialization point. Reserve processes for *isolation*
(crash boundaries), *serialization* (one writer), or *lifecycle* (something with a
start/stop).

### Supervisor strategies and restart types

Strategy (how siblings react when one child dies):

- `:one_for_one` — restart only the dead child. Default; use when children are independent.
- `:one_for_all` — restart **all** children. Use when they share fate (a pool that only makes sense whole).
- `:rest_for_one` — restart the dead child and everything **started after** it. Use for an ordered dependency chain (a connection, then the workers that use it).

Restart type (per child spec, `restart:`):

- `:permanent` — always restarted (default; long-lived servers).
- `:temporary` — never restarted (a one-shot job; its crash is fine).
- `:transient` — restarted only on **abnormal** exit; a `:normal`/`:shutdown` exit is left dead (a task that should retry on failure but stop cleanly on success).

Tune `max_restarts`/`max_seconds`: if a child crash-loops past the threshold the
supervisor itself gives up and escalates — that is the design, not a bug. Fix the
crash, don't raise the threshold blindly.

## Rustler NIFs (the Rust core)

The Rust crate lives under `native/<crate>/` with its own `Cargo.toml`. The Elixir
side:

```elixir
defmodule MyApp.Core do
  use Rustler, otp_app: :my_app, crate: "core"

  # Each NIF gets a stub that raises until the .so/.dll loads.
  def parse(_input), do: :erlang.nif_error(:nif_not_loaded)
end
```

Rust side (`native/core/src/lib.rs`):

```rust
#[rustler::nif]
fn parse(input: String) -> Result<String, String> { /* ... */ }

// Long or blocking work MUST run on a dirty scheduler.
#[rustler::nif(schedule = "DirtyCpu")]
fn heavy_transform(input: String) -> String { /* CPU-bound */ }

rustler::init!("Elixir.MyApp.Core");
```

**The single most important NIF rule: a NIF runs on the caller's scheduler thread
and there is no preemption.** A NIF that runs longer than ~1 ms on a normal
scheduler *stalls the BEAM* — it starves every process pinned to that scheduler and
breaks the runtime's soft-realtime guarantees. Anything non-trivial goes on
`DirtyCpu` (compute) or `DirtyIo` (blocking I/O). If work is truly long-running,
prefer chunking with `rustler::schedule` / yielding, or run it as a plain Rust
thread and message the result back. Never panic across the boundary — return
`Result` / `{:ok, _} | {:error, _}`.

## Mix releases (brief)

- `mix release` builds a self-contained artifact in `_build/prod/rel/<app>/`, including the ERTS. Run with `bin/<app> start` (foreground) or `daemon`.
- `config/config.exs` and `config/prod.exs` are **compile-time**. `config/runtime.exs` runs **at boot inside the release** — read `System.get_env/1` there, never at compile time, or the value bakes in at build.
- In Docker, build with a multi-stage image (`hexpm/elixir` builder → slim/distroless runtime), copy only `_build/prod/rel/<app>`, set `MIX_ENV=prod`. Rustler crates compile in the builder stage; the runtime image needs no Rust toolchain, only the produced `.so`.

## Quality gates

- `mix format` (config in `.formatter.exs`) — run in a pre-commit gate; `mix format --check-formatted` in CI.
- `mix credo --strict` — style and consistency lint.
- **Dialyzer** (via the `dialyxir` dep) plus `@spec`/`@type` typespecs. It uses *success typing*: it flags contradictions it can **prove** — a call that can never succeed, an unreachable clause, a `@spec` that conflicts with the code. It does **not** verify your specs are complete, catch logic errors, or reject code merely because it lacks a spec. Build the PLT once (`mix dialyzer --plt`), cache it in CI, keep it warm.
- Keep every gate under the 5-minute ceiling ([../../rules/timeouts.md](../../rules/timeouts.md)); a cold Dialyzer PLT build is the usual offender — cache the PLT, don't rebuild it per run.
- **Tests:** ExUnit with `async: true` where safe, the Ecto SQL Sandbox for DB isolation, Mox for behaviour mocks — keep them un-hangable and isolate global state, per [../../rules/testing-gates.md](../../rules/testing-gates.md) and [references/ecto-and-testing.md](references/ecto-and-testing.md).

## Common pitfalls — call these out sharply

- **The single-GenServer bottleneck.** One GenServer fronting a hot resource serializes *every* caller: `handle_call` processes one message at a time, so throughput is capped at one request's latency. Symptom: latency climbs with load while CPU sits idle. Fix: partition by key (a Registry-keyed process per entity), pool with `poolboy`/`NimblePool`, or push read state into ETS so reads bypass the process entirely. See [references/otp-patterns.md](references/otp-patterns.md).
- **Mailbox / message-queue overflow.** An unbounded producer casting to a slower consumer grows the mailbox without bound → memory blowup and ever-rising latency, because each `handle_info` must scan past a longer queue. `cast` gives you *no* back-pressure; `call` does (the caller blocks). Prefer `call`, or put a bounded stage (`GenStage`/`Broadway`) or an explicit bounded queue between them. Watch `Process.info(pid, :message_queue_len)`.
- **Hot code reload gotchas.** Releases support upgrades but they are fragile: changing a GenServer's state shape needs a `code_change/3`; a running process keeps its old closure until it restarts; `:permanent` children reload differently from dynamically-started ones. For most services, **prefer a rolling restart / blue-green deploy over live hot-upgrade** — it is far more predictable. Reserve hot reload for cases where dropping in-flight state is genuinely unacceptable.
