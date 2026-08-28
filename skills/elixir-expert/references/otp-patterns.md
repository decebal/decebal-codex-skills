# OTP patterns

Depth for the OTP section of the skill: a complete GenServer, a supervision tree,
Registry/DynamicSupervisor for many short-lived processes, back-pressure, and the
"don't build a bottleneck" rule with concrete fixes.

## GenServer skeleton

Every callback and the return shapes that matter.

```elixir
defmodule MyApp.Session do
  use GenServer

  # --- Client API (runs in the CALLER's process) ---
  def start_link(opts) do
    GenServer.start_link(__MODULE__, opts, name: opts[:name])
  end

  def fetch(pid), do: GenServer.call(pid, :fetch)          # blocks caller → back-pressure
  def touch(pid), do: GenServer.cast(pid, :touch)          # fire-and-forget → NO back-pressure

  # --- Server callbacks (run in the SERVER process) ---
  @impl true
  def init(opts) do
    # Return {:ok, state} fast. Do NOT do slow work here — it blocks the
    # supervisor's start. Defer it with {:continue, ...}.
    {:ok, %{data: nil, opts: opts}, {:continue, :load}}
  end

  @impl true
  def handle_continue(:load, state) do
    # Slow init work happens here, AFTER init returns and the process is registered.
    {:noreply, %{state | data: expensive_load(state.opts)}}
  end

  @impl true
  def handle_call(:fetch, _from, state) do
    {:reply, state.data, state}
    # Reply shapes: {:reply, r, state} | {:reply, r, state, timeout | {:continue, t}}
    #               {:noreply, state}  (reply later with GenServer.reply/2)
    #               {:stop, reason, reply, state}
  end

  @impl true
  def handle_cast(:touch, state), do: {:noreply, state}

  @impl true
  def handle_info(:timeout, state), do: {:stop, :normal, state}   # from a state timeout
  def handle_info(msg, state) do
    # Unexpected messages land here (monitors, raw sends). Match them or they pile up.
    require Logger
    Logger.debug("unexpected: #{inspect(msg)}")
    {:noreply, state}
  end

  @impl true
  def terminate(_reason, _state), do: :ok   # best-effort; NOT guaranteed on :brutal_kill
end
```

Notes that bite people:

- **`init/1` must be fast.** Anything slow blocks the supervisor's whole start sequence. Push it to `handle_continue/2` (the `{:continue, term}` from `init` runs before any other message).
- **`terminate/2` is best-effort.** It does not run on `:kill`/`:brutal_kill`, on a supervisor `:brutal_kill` shutdown, or if the process is not trapping exits for a linked crash. Don't rely on it for critical cleanup — use a linked resource or a monitor.
- **A state timeout** (`{:noreply, state, 5_000}`) delivers `:timeout` to `handle_info` after idle time — handy for idle-session eviction.

## Supervision tree example

```elixir
defmodule MyApp.Query.Supervisor do
  use Supervisor

  def start_link(arg), do: Supervisor.start_link(__MODULE__, arg, name: __MODULE__)

  @impl true
  def init(_arg) do
    children = [
      # rest_for_one: if the pool dies, restart the pool AND the workers after it,
      # but a worker crash does not take down the pool.
      {MyApp.Query.Pool, []},
      {MyApp.Query.CacheWarmer, []}
    ]

    Supervisor.init(children, strategy: :rest_for_one, max_restarts: 3, max_seconds: 5)
  end
end
```

A child spec can override restart type:

```elixir
Supervisor.child_spec({MyApp.OneShotJob, arg}, restart: :transient)
```

## Registry + DynamicSupervisor: many short-lived processes

The canonical shape for "a process per session / per connection / per entity". The
Registry gives each a name derived from its key; the DynamicSupervisor starts and
supervises them on demand.

```elixir
# In the app supervision tree:
#   {Registry, keys: :unique, name: MyApp.SessionRegistry},
#   {DynamicSupervisor, name: MyApp.SessionSup, strategy: :one_for_one}

defmodule MyApp.Session do
  use GenServer

  def via(id), do: {:via, Registry, {MyApp.SessionRegistry, id}}

  def start(id) do
    DynamicSupervisor.start_child(MyApp.SessionSup, {__MODULE__, id})
  end

  def start_link(id), do: GenServer.start_link(__MODULE__, id, name: via(id))

  # Look up or start — the Registry makes "already running" a fast, race-safe check.
  def whereis(id) do
    case Registry.lookup(MyApp.SessionRegistry, id) do
      [{pid, _}] -> {:ok, pid}
      [] -> start(id)
    end
  end

  @impl true
  def init(id), do: {:ok, %{id: id}}
end
```

Why this beats one big GenServer with a `Map` of sessions: each session is an
**isolated crash boundary** (one bad session can't corrupt the others), work runs
**concurrently** across sessions, and the DynamicSupervisor handles lifecycle. The
Registry entry is removed automatically when the process dies.

## Back-pressure

Concurrency without a bound is a memory leak waiting for load. Three ways to get a
bound, cheapest first:

1. **Use `call`, not `cast`.** A `GenServer.call` blocks the caller until the server replies, so a slow server naturally throttles its callers. `cast` never does — the mailbox just grows.
2. **Bounded concurrent map:** `Task.async_stream/3` with `max_concurrency:` processes an enumerable with a hard cap and applies back-pressure to the producer.
   ```elixir
   items
   |> Task.async_stream(&work/1, max_concurrency: 10, timeout: 30_000)
   |> Enum.to_list()
   ```
3. **A demand-driven pipeline:** `GenStage` (producer → consumer, consumer pulls a bounded number of events), `Flow` (parallel data processing over GenStage), or `Broadway` (batteries-included ingestion with rate-limiting and batching). Reach for these when a stage needs its own supervision, partitioning, or a rate limit — not for a one-off parallel map, where `Task.async_stream` is simpler.

Instrument the queue you're worried about:

```elixir
{:message_queue_len, n} = Process.info(pid, :message_queue_len)
```

## The "don't build a bottleneck" rule

**A GenServer serializes every message it receives.** That is its feature (a single
writer, ordered) and its trap (one hot server caps throughput at one message's
service time). Diagnosis: latency rises with concurrency while a core sits idle, and
`:message_queue_len` climbs.

Fixes, in order of preference:

- **Partition by key.** Instead of one server, a Registry-keyed process per entity (the pattern above). Independent keys now run in parallel; the ordering guarantee is preserved *per key*, which is usually all you needed.
- **Push read-heavy state into ETS.** A public `:ets` table (`:read_concurrency`/`:write_concurrency`) lets readers hit the table directly with no message round-trip; the GenServer stays the single **writer**. This removes the read bottleneck entirely.
- **Pool the workers.** `poolboy` or `NimblePool` front a fixed set of identical workers (DB connections, NIF handles) so N callers share N workers with checkout/checkin — bounded concurrency and no single funnel.

Reserve a single GenServer for things that genuinely must be serialized globally
(a rate-limiter token bucket, a monotonic counter, a single external connection).
Everything else partitions.
