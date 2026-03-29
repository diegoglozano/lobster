# Lobster

A low-latency limit order book and matching engine written in Rust.

Built to demonstrate production-grade Rust across the stack: correct data structures, lock-free concurrency patterns, zero-copy serialization, and a clean crate architecture.

---

## What it does

An order book is the core data structure of any financial exchange — it tracks outstanding buy and sell orders and matches them when their prices cross.

Lobster implements:

- **A matching engine** that pairs bids and asks using price-time priority (FIFO within each price level)
- **Single-writer concurrency** via `crossbeam` channels — the order book is owned by one thread, callers submit orders through a lock-free message queue
- **Zero-copy serialization** via FlatBuffers — orders are encoded into a compact binary wire format with no heap allocations on the read path
- **Async TCP server** via Tokio — accepts FlatBuffers-encoded orders over TCP, routes to the matching engine, sends back execution reports
- **Trade persistence** via `sqlx` + PostgreSQL — every match is asynchronously persisted without blocking the matching engine

---

## Architecture

```
lobster/
├── crates/
│   ├── lobster-core      # Order book data structure and matching logic
│   ├── lobster-engine    # Concurrent matching engine (single-writer, channel-based)
│   ├── lobster-proto     # FlatBuffers wire format and conversion layer
│   └── lobster-db        # PostgreSQL persistence layer (sqlx)
└── bins/
    ├── lobster-server    # TCP server accepting order messages
    └── lobster-client    # Test client for manual end-to-end testing
```

### `lobster-core`

The pure domain layer. No I/O, no threading.

- `Order` — id, side, type, price (`rust_decimal::Decimal`), units, timestamp
- `OrderBook` — two `BTreeMap<Price, VecDeque<Order>>` — one for bids, one for asks
- `add_order()` — inserts an order into the correct price level using the `entry()` API
- `match_orders()` — walks the best bid and best ask, handles exact fills, partial fills, and no-match. Cleans up empty price levels after each match.

`BTreeMap` gives O(log n) insert and O(1) access to the best bid/ask via `last_key_value()` / `first_key_value()`. `VecDeque` maintains FIFO order within each price level.

### `lobster-engine`

The concurrency layer. Wraps `lobster-core` in a single-writer design:

- `MatchingEngine` owns the `OrderBook` exclusively — no locks, no shared mutable state
- Callers hold an `EngineHandle` and call `handle.submit(order)` from any thread
- Each submission creates a one-shot response channel, sends the order to the engine thread, and blocks until the result comes back
- The engine thread runs a `while let` loop: receive → `add_order` → `match_orders` → reply

This avoids `RwLock` contention entirely. Under high throughput, a lock-based approach degrades at p99 as threads queue for the lock. The single-writer pattern keeps tail latency flat.

### `lobster-proto`

The serialization layer. Uses FlatBuffers for zero-copy binary encoding:

- `order.fbs` defines the wire schema — enums backed by `byte`, price as `string` (preserves `Decimal` precision), timestamp as `int64` nanoseconds
- `to_fb_order()` — serializes a `lobster_core::Order` to `Vec<u8>`
- `to_core_order()` — deserializes a FlatBuffers `Order` back to `lobster_core::Order`, preserving `id` and `timestamp` via `Order::from_parts()`

FlatBuffers reads fields directly from the byte buffer via table offsets — no intermediate struct, no allocation on the read path.

### `lobster-db`

The persistence layer. Uses `sqlx` for async PostgreSQL access:

- `TradeRepository` — owns a `PgPool` connection pool
- `run_migrations()` — applies SQL migrations via `sqlx::migrate!`
- `insert_trade()` — persists a `Trade` to the `trades` table
- Integration tested with `testcontainers` — each test spins up a real Postgres container, runs migrations, and tears down automatically

### `lobster-server`

The TCP server. Glues all crates together:

- Binds on port 7777, accepts connections via Tokio
- Each connection gets its own async task — reads FlatBuffers-encoded orders in a loop
- Orders are deserialized via `lobster-proto` and submitted to the engine via `EngineHandle`
- `handle.submit()` runs on a `spawn_blocking` thread to avoid blocking the Tokio runtime
- Matches are sent back to the client as `1u8` + two 16-byte UUIDs; no-match as `0u8`
- A dedicated Tokio task drains the trade channel and persists each trade to Postgres asynchronously — the engine never waits for the DB write

### `lobster-client`

A minimal test client for manual end-to-end testing:

- Connects to the server, sends a FlatBuffers-encoded ask then a matching bid
- Reads and prints the response for each order

---

## Key design decisions

**`rust_decimal` over `f64` for price**

Floating point arithmetic is unsuitable for financial math. `Decimal` gives exact representation and implements `Ord`, which is required for use as a `BTreeMap` key.

**Single-writer over `RwLock`**

`Arc<RwLock<OrderBook>>` is the obvious first approach but introduces lock contention under load. The channel-based single-writer pattern eliminates shared mutable state — the matching engine is inherently single-threaded, and the channel is the only synchronization point.

**FlatBuffers over JSON / bincode**

JSON is human-readable but slow to parse and verbose on the wire. `bincode` is fast but Rust-specific and still allocates. FlatBuffers reads directly from the buffer with no parsing step and no allocation — fields are accessed via byte offsets into the original buffer.

**Decoupled DB writes via channel**

Persisting a trade to Postgres takes milliseconds — orders of magnitude slower than matching in memory. Writing synchronously inside the engine would destroy tail latency. Instead, the engine sends matched trades into a `tokio::sync::mpsc` channel and returns immediately. A separate async task drains the channel and batches writes to Postgres. The engine never waits for I/O.

---

## Running tests

```bash
cargo test
```

```bash
cargo clippy
```

---

## Running

Start Postgres (requires Colima or Docker):
```bash
colima start
DATABASE_URL=postgres://postgres:postgres@localhost:5432/lobster cargo run --bin lobster-server
```

In another terminal:
```bash
cargo run --bin lobster-client
```

## Running tests

```bash
cargo test        # unit + integration tests (requires Colima/Docker for lobster-db)
cargo clippy      # lints
```

---

## Roadmap

- [x] Phase 1 — Core order book and matching engine (`lobster-core`)
- [x] Phase 2 — Single-writer concurrency via channels (`lobster-engine`)
- [x] Phase 3 — FlatBuffers zero-copy serialization (`lobster-proto`)
- [x] Phase 4 — Async TCP server and test client (`lobster-server`, `lobster-client`)
- [ ] Phase 5 — REST API with `axum` for querying trade history (`lobster-api`)
- [ ] Phase 6 — Structured logging with `tracing`, Prometheus metrics, Dockerfile + Grafana dashboard
