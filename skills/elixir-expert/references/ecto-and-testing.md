# Ecto and testing

Depth for persistence and tests: schema + migration + changeset, `Repo`
transactions and `Ecto.Multi`, ExUnit `async`, Mox behaviour mocks, the Ecto SQL
Sandbox, and test containers for real dependencies.

Tests must be **un-hangable** and must **isolate global state** — see
[../../../rules/testing-gates.md](../../../rules/testing-gates.md). The Sandbox and
`async` rules below are the Ecto-specific application of that.

## Schema + migration + changeset

Migration (`priv/repo/migrations/<ts>_create_users.exs`):

```elixir
defmodule MyApp.Repo.Migrations.CreateUsers do
  use Ecto.Migration

  def change do
    create table(:users) do
      add :email, :string, null: false
      add :name, :string
      timestamps()
    end

    create unique_index(:users, [:email])
  end
end
```

Schema — a domain type, pure, no I/O:

```elixir
defmodule MyApp.Accounts.User do
  use Ecto.Schema
  import Ecto.Changeset

  schema "users" do
    field :email, :string
    field :name, :string
    timestamps()
  end

  def changeset(user, attrs) do
    user
    |> cast(attrs, [:email, :name])
    |> validate_required([:email])
    |> validate_format(:email, ~r/@/)
    # unique_constraint pairs with the DB unique_index: it turns the DB's
    # constraint violation into a changeset error instead of an exception.
    |> unique_constraint(:email)
  end
end
```

`cast/3` whitelists which attrs may be set (never `cast` an unfiltered params map
with a `:role`/`:admin` field — that is a mass-assignment hole). `validate_*` run in
memory; `*_constraint` run against the DB on insert/update.

## Repo transactions and Ecto.Multi

For a single guarded operation, `Repo.transaction/1` with a function is fine. For a
sequence where a later step depends on an earlier one and any failure must roll back
**everything**, use `Ecto.Multi` — it names each step, threads results forward, and
returns which step failed without nesting:

```elixir
alias Ecto.Multi

Multi.new()
|> Multi.insert(:user, User.changeset(%User{}, attrs))
|> Multi.insert(:profile, fn %{user: user} ->
  Profile.changeset(%Profile{}, %{user_id: user.id})
end)
|> Multi.run(:notify, fn _repo, %{user: user} ->
  MyApp.Mailer.welcome(user)   # must return {:ok, _} | {:error, _}
end)
|> MyApp.Repo.transaction()
|> case do
  {:ok, %{user: user}} -> {:ok, user}
  {:error, step, reason, _changes_so_far} -> {:error, step, reason}
end
```

Everything inside one `Repo.transaction` runs on **one connection** — do not spawn
tasks that call the Repo from inside it (they'd check out a different connection and
sit outside the transaction).

## ExUnit and `async: true`

```elixir
defmodule MyApp.AccountsTest do
  use MyApp.DataCase, async: true   # DataCase sets up the Sandbox (below)

  test "rejects a blank email" do
    assert {:error, changeset} = MyApp.Accounts.create_user(%{email: ""})
    assert "can't be blank" in errors_on(changeset).email
  end
end
```

`async: true` runs this module concurrently with other async modules — a real
speedup. It is **safe only when the test touches no shared global mutable state**.
Unsafe (`async: false`, or redesign) when a test:

- mutates `Application.put_env/3` / other process-global config,
- registers or relies on a **named** process or a global ETS table,
- talks to a shared external resource without isolation,
- uses `Mox` in global mode (see below).

DB access is *not* on that list **because** the SQL Sandbox isolates it per test —
that is the whole point of the Sandbox.

## Ecto SQL Sandbox (DB isolation)

The Sandbox wraps each test in a transaction that is rolled back at the end, so
concurrent async tests never see each other's rows and the DB returns to a clean
state with no manual teardown.

`test/test_helper.exs`:

```elixir
ExUnit.start()
Ecto.Adapters.SQL.Sandbox.mode(MyApp.Repo, :manual)
```

The `DataCase` `setup` checks out a connection per test and, for a test that spawns
processes needing DB access, shares it:

```elixir
setup tags do
  pid = Ecto.Adapters.SQL.Sandbox.start_owner!(MyApp.Repo, shared: not tags[:async])
  on_exit(fn -> Ecto.Adapters.SQL.Sandbox.stop_owner(pid) end)
  :ok
end
```

`shared: true` (used only for `async: false` tests) lets any process see the
checked-out connection. For an `async: true` test that spawns a Task hitting the
Repo, keep it non-shared and explicitly `Ecto.Adapters.SQL.Sandbox.allow(Repo,
self(), child_pid)` — sharing in an async test would leak the connection across
concurrent tests.

## Mox: behaviour mocks

Mox mocks a **behaviour**, not a module — so the mock is verified against a real
contract and drifts when the contract changes. Define the behaviour, inject it via
config, mock it in tests.

```elixir
# 1. The contract (in lib/), an infrastructure boundary you want to fake in tests:
defmodule MyApp.Weather do
  @callback fetch(String.t()) :: {:ok, map()} | {:error, term()}
end

# 2. config/test.exs — swap the real adapter for the mock:
#    config :my_app, :weather, MyApp.WeatherMock

# 3. test/test_helper.exs — define the mock from the behaviour:
Mox.defmock(MyApp.WeatherMock, for: MyApp.Weather)
```

In the test:

```elixir
import Mox
setup :verify_on_exit!   # fails the test if an `expect` was not called

test "summarizes weather" do
  expect(MyApp.WeatherMock, :fetch, fn "London" -> {:ok, %{temp_c: 12}} end)
  assert {:ok, "12°C"} = MyApp.Report.summary("London")
end
```

- **`expect/4`** sets a call that *must* happen (and how many times); `verify_on_exit!` enforces it. **`stub/3`** provides a default with no call-count requirement.
- Mox is **process-based** by default: the expectation is visible only to the process that set it. If the code under test runs the call in a spawned Task/GenServer, either `allow(MyApp.WeatherMock, self(), child_pid)` or use `set_mox_global` — and a global-mode test **must** be `async: false`, since global expectations bleed across concurrent tests.

## Test containers (real dependencies)

Sandbox and Mox cover the DB and pure boundaries. When a test needs a **real**
dependency the code can't fake honestly — a specific Postgres extension, Redis,
Kafka, an S3-compatible store — use the `testcontainers` Hex package to boot a
throwaway container per suite:

- Start the container in a `setup_all` (once per module), read its mapped host/port, point the client at it, stop it on exit.
- **Keep it un-hangable:** give container startup and every client call an explicit timeout, and always register teardown with `on_exit`/`ExUnit.after_suite` so a failed test can't leak a running container that wedges the next run. Never `Process.sleep` waiting for readiness — poll a health check with a bounded number of attempts.
- These tests are slower and hit a real socket, so keep them **`async: false`** and few; put the fast, isolated logic under Sandbox + Mox. A container-backed suite that boots one image and runs in seconds is fine; a per-test container is a timeout waiting to happen.

See [../../../rules/testing-gates.md](../../../rules/testing-gates.md) for the
un-hangable and isolate-global-state gates these rules satisfy.
