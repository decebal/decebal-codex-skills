# C4 + Mermaid templates (copy-paste)

Copy the block for the level you need, then replace every placeholder with a real
component name and a real technology or relationship. Each structural level has a
`C4*` template AND a plain-diagram fallback — read the caveat first to choose.

## Contents

- [Caveat: C4 types are experimental](#caveat-c4-types-are-experimental)
- [L1 — System Context](#l1--system-context)
- [L2 — Container](#l2--container)
- [L3 — Component](#l3--component)
- [L4 — Code (usually skip)](#l4--code-usually-skip)
- [Data flow — sequence diagram](#data-flow--sequence-diagram)
- [Embedding in a .md](#embedding-in-a-md)

## Caveat: C4 types are experimental

The `C4Context` / `C4Container` / `C4Component` diagram types are marked
experimental in Mermaid: auto-layout is unstable, spacing is hard to control, and
output varies across Mermaid versions and renderers. Use them when you want C4's
labelled shapes and boundaries out of the box. When you need reliable layout or
maximum renderer portability, use the **plain fallback** (`flowchart` / `graph` /
`sequenceDiagram`) — those are stable everywhere GitHub-flavored Mermaid renders.
The fallbacks below reproduce the same structure with ordinary shapes.

---

## L1 — System Context

The system as one box, plus the people and external systems it interacts with.
Keep it to under ~10 boxes.

### C4Context

```mermaid
C4Context
    title System Context — Order Platform
    Person(customer, "Customer", "Places and tracks orders")
    Person(ops, "Ops Engineer", "Monitors and operates the platform")
    System(platform, "Order Platform", "Lets customers place, pay for, and track orders")
    System_Ext(stripe, "Stripe", "Processes card payments")
    System_Ext(email, "Email Provider", "Sends transactional email")
    Rel(customer, platform, "Places orders via", "HTTPS")
    Rel(ops, platform, "Operates and observes")
    Rel(platform, stripe, "Charges cards via", "HTTPS/JSON")
    Rel(platform, email, "Sends receipts via", "SMTP")
    UpdateLayoutConfig($c4ShapeInRow="3", $c4BoundaryInRow="1")
```

### Plain fallback (flowchart)

```mermaid
flowchart TB
    customer(["Customer<br/><i>person</i>"])
    ops(["Ops Engineer<br/><i>person</i>"])
    platform["Order Platform<br/><i>software system</i>"]
    stripe["Stripe<br/><i>external system</i>"]
    email["Email Provider<br/><i>external system</i>"]

    customer -->|"places orders via — HTTPS"| platform
    ops -->|"operates & observes"| platform
    platform -->|"charges cards via — HTTPS/JSON"| stripe
    platform -->|"sends receipts via — SMTP"| email
```

*Caption example: L1 System Context. Rounded = people, plain boxes = software systems, arrows = interactions labelled with protocol.*

---

## L2 — Container

Zoom into the one system from L1. Each box is a separately deployable/runnable
thing: a web app, an API service, a datastore, a message broker.

### C4Container

```mermaid
C4Container
    title Container — Order Platform
    Person(customer, "Customer", "Places and tracks orders")
    Container_Boundary(platform, "Order Platform") {
        Container(spa, "Web App", "TypeScript, React", "Order UI served to the browser")
        Container(api, "API Service", "Rust, Axum", "Order, payment, and account endpoints")
        Container(worker, "Fulfilment Worker", "Rust", "Consumes order events, coordinates shipping")
        ContainerQueue(bus, "Event Bus", "NATS", "Carries domain events between services")
        ContainerDb(db, "Orders DB", "PostgreSQL", "Orders, payments, accounts")
    }
    System_Ext(stripe, "Stripe", "Processes card payments")

    Rel(customer, spa, "Uses", "HTTPS")
    Rel(spa, api, "Calls", "HTTPS/JSON")
    Rel(api, db, "Reads/writes", "SQL")
    Rel(api, bus, "Publishes order events to")
    Rel(worker, bus, "Subscribes to")
    Rel(api, stripe, "Charges via", "HTTPS/JSON")
    UpdateLayoutConfig($c4ShapeInRow="2")
```

### Plain fallback (graph LR)

```mermaid
graph LR
    customer(["Customer"])
    stripe["Stripe<br/><i>external</i>"]

    subgraph platform["Order Platform"]
        spa["Web App<br/>[TypeScript · React]"]
        api["API Service<br/>[Rust · Axum]"]
        worker["Fulfilment Worker<br/>[Rust]"]
        bus[["Event Bus<br/>[NATS]"]]
        db[("Orders DB<br/>[PostgreSQL]")]
    end

    customer -->|HTTPS| spa
    spa -->|"HTTPS/JSON"| api
    api -->|SQL| db
    api -->|"publishes order events"| bus
    bus -->|"delivers events"| worker
    api -->|"charges — HTTPS/JSON"| stripe
```

*Caption example: L2 Container view. Boxes = deployable units, `[[…]]` = message bus, cylinder = datastore; arrows labelled with protocol, synchronous unless noted.*

---

## L3 — Component

Zoom into ONE container from L2 (here, the API Service). Boxes are the major
internal parts — modules, handlers, repositories — not classes.

### C4Component

```mermaid
C4Component
    title Component — API Service
    Container_Boundary(api, "API Service") {
        Component(router, "HTTP Router", "Axum", "Routes requests, auth middleware")
        Component(orders, "Orders Handler", "Rust module", "Validates and orchestrates order use-cases")
        Component(payments, "Payments Handler", "Rust module", "Orchestrates charge/refund use-cases")
        Component(repo, "Order Repository", "Rust module", "Persists and loads orders")
        Component(stripeClient, "Stripe Client", "Rust module", "Wraps the Stripe HTTP API")
    }
    ContainerDb(db, "Orders DB", "PostgreSQL", "Orders, payments, accounts")
    System_Ext(stripe, "Stripe", "Payment gateway")

    Rel(router, orders, "Dispatches to")
    Rel(router, payments, "Dispatches to")
    Rel(orders, repo, "Uses")
    Rel(payments, stripeClient, "Uses")
    Rel(repo, db, "Reads/writes", "SQL")
    Rel(stripeClient, stripe, "Calls", "HTTPS/JSON")
```

### Plain fallback (flowchart)

```mermaid
flowchart TB
    subgraph api["API Service"]
        router["HTTP Router<br/>[Axum]"]
        orders["Orders Handler"]
        payments["Payments Handler"]
        repo["Order Repository"]
        stripeClient["Stripe Client"]
    end
    db[("Orders DB<br/>[PostgreSQL]")]
    stripe["Stripe<br/><i>external</i>"]

    router --> orders
    router --> payments
    orders -->|uses| repo
    payments -->|uses| stripeClient
    repo -->|SQL| db
    stripeClient -->|"HTTPS/JSON"| stripe
```

*Caption example: L3 Component view of the API Service. Boxes = internal modules; only this container's insides are shown, neighbours are drawn as context.*

---

## L4 — Code (usually skip)

C4 level 4 shows classes/functions inside one component. It duplicates what an IDE
or `cargo doc` already shows and goes stale fast — **default to omitting it.** Draw
one only for a genuinely intricate design worth freezing in prose. Mermaid has no
`C4Code` type; use a `classDiagram`:

```mermaid
classDiagram
    class OrderService {
        +place(cmd: PlaceOrder) Result~OrderId~
        +cancel(id: OrderId) Result~()~
    }
    class OrderRepository {
        <<trait>>
        +save(order: Order) Result~()~
        +load(id: OrderId) Result~Order~
    }
    class PgOrderRepository
    OrderService --> OrderRepository : depends on
    OrderRepository <|.. PgOrderRepository : implements
```

---

## Data flow — sequence diagram

For ONE scenario, show the ordered messages between containers/components over
time. `autonumber` numbers the steps; `activate`/`deactivate` (or `+`/`-`) show
who is doing work; `alt`/`opt`/`loop` express branching.

```mermaid
sequenceDiagram
    autonumber
    actor Customer
    participant SPA as Web App
    participant API as API Service
    participant DB as Orders DB
    participant Stripe

    Customer->>SPA: Submit checkout
    SPA->>+API: POST /orders
    API->>+DB: INSERT order (pending)
    DB-->>-API: order id
    API->>+Stripe: POST /charges
    alt charge succeeds
        Stripe-->>-API: 200 charge ok
        API->>DB: UPDATE order = paid
        API-->>SPA: 201 Created + order id
        SPA-->>Customer: Show confirmation
    else charge declined
        API->>DB: UPDATE order = failed
        API-->>-SPA: 402 Payment Required
        SPA-->>Customer: Show decline message
    end
```

*Caption example: Checkout data flow. Vertical = time downward, solid arrow = request, dashed = response; the `alt` block shows the success vs. declined paths.*

---

## Embedding in a .md

Paste the chosen diagram into a fenced block tagged `mermaid`, under a heading that
names the level, with an italic caption carrying the legend:

````markdown
## System Context

```mermaid
flowchart TB
    ...
```
*L1 System Context. Rounded = people, boxes = systems, arrows labelled with protocol.*
````

GitHub, GitLab, Obsidian, and most static-site generators render `mermaid` blocks
inline. To preview locally, use the Mermaid CLI (`mmdc`) or paste into the Mermaid
Live Editor.
