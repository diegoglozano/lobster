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

---

## Architecture

```
lobster/
├── crates/
│   ├── lobster-core      # Order book data structure and matching logic
│   ├── lobster-engine    # Concurrent matching engine (single-writer, channel-based)
│   └── lobster-proto     # FlatBuffers wire format and conversion layer
└── bins/
    └── lobster-server    # (in progress) TCP server accepting order messages
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

---

## Key design decisions

**`rust_decimal` over `f64` for price**

Floating point arithmetic is unsuitable for financial math. `Decimal` gives exact representation and implements `Ord`, which is required for use as a `BTreeMap` key.

**Single-writer over `RwLock`**

`Arc<RwLock<OrderBook>>` is the obvious first approach but introduces lock contention under load. The channel-based single-writer pattern eliminates shared mutable state — the matching engine is inherently single-threaded, and the channel is the only synchronization point.

**FlatBuffers over JSON / bincode**

JSON is human-readable but slow to parse and verbose on the wire. `bincode` is fast but Rust-specific and still allocates. FlatBuffers reads directly from the buffer with no parsing step and no allocation — fields are accessed via byte offsets into the original buffer.

---

## Running tests

```bash
cargo test
```

```bash
cargo clippy
```

---

## Roadmap

- [ ] Phase 4 — Tokio TCP server accepting FlatBuffers-encoded order messages, routing to the matching engine via channels, sending back execution reports
- [ ] Phase 5 — Trade persistence with `sqlx` + PostgreSQL, REST API with `axum`
- [ ] Phase 6 — Structured logging with `tracing`, Prometheus metrics, Dockerfile + Grafana dashboard
