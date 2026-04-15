# Diesel-Async: TLS & Connection Strategies

---

## Table of Contents

- [The Problem](<diesel_async_tls_and_connection_strategies#The Problem>)
- [How TLS Works with `AsyncPgConnection`](<diesel_async_tls_and_connection_strategies#How TLS Works with `AsyncPgConnection`>)
  - [The Default: No TLS](<diesel_async_tls_and_connection_strategies#The Default: No TLS>)
  - [Official Documentation on TLS](<diesel_async_tls_and_connection_strategies#Official Documentation on TLS>)
  - [The Key Method: `try_from_client_and_connection`](<diesel_async_tls_and_connection_strategies#The Key Method: `try_from_client_and_connection`>)
  - [Required Dependencies](<diesel_async_tls_and_connection_strategies#Required Dependencies>)
  - [Establishing a TLS Connection](<diesel_async_tls_and_connection_strategies#Establishing a TLS Connection>)
  - [Pooled TLS Connections with Deadpool](<diesel_async_tls_and_connection_strategies#Pooled TLS Connections with Deadpool>)
- [Alternative: `SyncConnectionWrapper<PgConnection>`](<diesel_async_tls_and_connection_strategies#Alternative: `SyncConnectionWrapper<PgConnection>`>)
  - [Why This Works](#why-this-works)
  - [Setup](#setup)
  - [Pooled Connection with Deadpool](#pooled-connection-with-deadpool)
  - [API Changes](#api-changes)
- [Comparison: `AsyncPgConnection` vs `SyncConnectionWrapper<PgConnection>`](#comparison-asyncpgconnection-vs-syncconnectionwrapperpgconnection)
- [Appendix: `sync-connection-wrapper` vs `async-connection-wrapper` Features](#appendix-sync-connection-wrapper-vs-async-connection-wrapper-features)
- [Appendix: Do You Need Explicit Dependencies?](#appendix-do-you-need-explicit-dependencies)

---

## The Problem

When using `diesel-async` with the `postgres` + `deadpool` features, there is **no TLS flag** on
`diesel-async` or `diesel` that automatically enables SSL/TLS for database connections.

In production, most managed databases (AWS RDS, DigitalOcean, Supabase, etc.) **require** SSL.
So how do you connect?

There are two strategies:

1. **Use `AsyncPgConnection` with manual TLS setup** (pure Rust, more boilerplate)
2. **Use `SyncConnectionWrapper<PgConnection>`** which delegates to `libpq`, where TLS is just a
   connection string parameter (simpler, but introduces a C system dependency)

---

## How TLS Works with `AsyncPgConnection`

### The Default: No TLS

`AsyncPgConnection::establish()` internally calls `tokio_postgres::connect(url, NoTls)`.

**There is no hidden flag, no auto-detection, no feature you can enable.**

Even if your connection string contains `?sslmode=require`, it will either be ignored or the
connection will fail — because no TLS implementation has been provided.

### Official Documentation on TLS

From the [`AsyncPgConnection` docs](https://docs.rs/diesel-async/latest/diesel_async/struct.AsyncPgConnection.html#tls):

> **§TLS**
>
> Connections created by `AsyncPgConnection::establish` **do not support TLS**.
>
> TLS support for tokio_postgres connections is implemented by **external crates**,
> e.g. `tokio_postgres_rustls`.
>
> `AsyncPgConnection::try_from_client_and_connection` can be used to construct a connection
> from an existing `tokio_postgres::Connection` with TLS enabled.

### The Key Method: `try_from_client_and_connection`

This is the designated constructor for TLS connections:

```rust
pub async fn try_from_client_and_connection<S>(
    client: Client,
    conn: Connection<Socket, S>,
) -> ConnectionResult<Self>
where
    S: TlsStream + Unpin + Send + 'static,
```

You create a TLS-enabled `tokio_postgres` connection yourself, then hand **both** the `Client`
and the `Connection` to this method. Diesel-async handles spawning the connection future
internally — you do **not** need to `tokio::spawn` it yourself.

This is different from `try_from(client)` which only takes a `Client` and would require you
to spawn the connection future manually.

### Required Dependencies

```toml
[dependencies]
diesel = { version = "2.3.7", features = ["chrono", "uuid"] }
diesel-async = { version = "0.8.0", features = [
  "deadpool",
  "migrations",
  "postgres"
] }

# Required for TLS with AsyncPgConnection:
tokio-postgres = "0.7"              # for tokio_postgres::connect()
tokio-postgres-rustls = "0.13"      # for MakeRustlsConnect
rustls = "0.23"                     # for ClientConfig builder
webpki-roots = "1.0"                # for TLS_SERVER_ROOTS (Mozilla root CAs)
```

**Why are all four needed explicitly?**

- `tokio-postgres` is a transitive dependency of `diesel-async`, but in Rust 2021+ edition
  you cannot use a transitive dependency's API without listing it in your own `Cargo.toml`.
  You need `tokio_postgres::connect()` which is a free function — not re-exported by diesel-async.
- `tokio-postgres-rustls` is a completely external crate. Diesel-async has no dependency on it.
- `rustls` is needed because you construct `rustls::ClientConfig` directly in your code.
- `webpki-roots` is needed for Mozilla's root CA certificates (`TLS_SERVER_ROOTS`).

Cargo will unify overlapping transitive versions automatically — you won't get duplicate copies
of `tokio-postgres` or `rustls` in your build.

### Establishing a TLS Connection

```rust
use diesel::ConnectionResult;
use diesel_async::AsyncPgConnection;
use rustls::RootCertStore;
use tokio_postgres_rustls::MakeRustlsConnect;

pub async fn establish_tls(database_url: &str) -> ConnectionResult<AsyncPgConnection> {
    // 1. Build a rustls ClientConfig with Mozilla root certificates
    let root_store = RootCertStore::from_iter(
        webpki_roots::TLS_SERVER_ROOTS.iter().cloned(),
    );
    let rustls_config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let tls = MakeRustlsConnect::new(rustls_config);

    // 2. Connect using tokio-postgres directly (with TLS!)
    let (client, connection) = tokio_postgres::connect(database_url, tls)
        .await
        .map_err(|e| diesel::ConnectionError::BadConnection(e.to_string()))?;

    // 3. Pass BOTH client and connection — diesel-async handles spawning
    AsyncPgConnection::try_from_client_and_connection(client, connection).await
}
```

### Pooled TLS Connections with Deadpool

To use TLS with a deadpool connection pool, wire the custom TLS setup into
`ManagerConfig::custom_setup`:

```rust
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::{AsyncDieselConnectionManager, ManagerConfig};

pub fn create_pool(database_url: &str) -> Pool<AsyncPgConnection> {
    let mut config = ManagerConfig::default();
    config.custom_setup = Box::new(|url| {
        Box::pin(establish_tls(url))  // reuse the function from above
    });

    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new_with_config(
        database_url,
        config,
    );

    Pool::builder(manager)
        .build()
        .expect("Failed to create connection pool")
}
```

Every connection the pool creates will now use TLS transparently.

---

## Alternative: `SyncConnectionWrapper<PgConnection>`

### Why This Works

Diesel's sync `PgConnection` uses **`libpq`** — PostgreSQL's official C client library
(via the `pq-sys` crate). `libpq` handles SSL/TLS **natively**. You just set connection
string parameters:

```text
postgres://user:pass@host:5432/dbname?sslmode=require
```

That's it. No extra crates. No custom setup functions. `libpq` negotiates TLS using the
system's OpenSSL.

The `sync-connection-wrapper` feature in diesel-async wraps this sync `PgConnection` to
implement `AsyncConnection`. Internally, it runs blocking database operations on tokio's
blocking thread pool (via `spawn_blocking`).

### Setup

```toml
[dependencies]
# Need the postgres feature on diesel itself (pulls in libpq via pq-sys)
diesel = { version = "2.3.7", features = ["postgres", "chrono", "uuid"] }
diesel-async = { version = "0.8.0", features = [
  "deadpool",
  "sync-connection-wrapper",   # <-- enables SyncConnectionWrapper
  "postgres",
] }

# That's it. No tokio-postgres, no rustls, no webpki-roots.
```

**System requirement:** You need `libpq-dev` (Debian/Ubuntu), `postgresql-devel` (Fedora/RHEL),
or equivalent installed on your system and in your Docker image.

### Pooled Connection with Deadpool

```rust
use diesel::PgConnection;
use diesel_async::sync_connection_wrapper::SyncConnectionWrapper;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;

// sslmode=require in the URL is all you need for TLS!
let database_url = "postgres://user:pass@host:5432/db?sslmode=require";
let manager = AsyncDieselConnectionManager::<SyncConnectionWrapper<PgConnection>>::new(database_url);
let pool = Pool::builder(manager)
    .build()
    .expect("Failed to create pool");
```

No `ManagerConfig::custom_setup`. No extra crates. Just `::new()` with a connection URL.

### API Changes

The query DSL stays **exactly the same**. The only change is your type annotations:

```rust
// Before (pure async, no TLS without custom setup)
type DbPool = Pool<AsyncPgConnection>;

// After (sync wrapper, TLS works via connection string)
type DbPool = Pool<SyncConnectionWrapper<PgConnection>>;

// Query code is IDENTICAL in both cases:
let results = users::table
    .filter(users::active.eq(true))
    .load::<User>(&mut conn)
    .await?;
```

---

## Comparison: `AsyncPgConnection` vs `SyncConnectionWrapper<PgConnection>`

| Aspect                     | `AsyncPgConnection`                               | `SyncConnectionWrapper<PgConnection>`                   |
| -------------------------- | ------------------------------------------------- | ------------------------------------------------------- |
| **TLS setup**              | Manual (4 extra crates, ~40 lines of custom code) | Just `?sslmode=require` in URL                          |
| **Underlying library**     | `tokio-postgres` (pure Rust)                      | `libpq` (C library via `pq-sys`)                        |
| **System dependency**      | None                                              | `libpq-dev` must be installed                           |
| **How queries execute**    | Truly async on the tokio event loop               | `spawn_blocking` — uses tokio's blocking thread pool    |
| **Pipelining**             | ✅ Yes (concurrent queries on one connection)     | ❌ No                                                   |
| **Per-query overhead**     | Minimal                                           | Thread context switch (negligible vs actual DB latency) |
| **Setup complexity**       | Higher, especially with TLS                       | Much simpler                                            |
| **Maturity**               | tokio-postgres is solid but newer                 | `libpq` has decades of production use                   |
| **Pooling (deadpool/bb8)** | ✅ Yes                                            | ✅ Yes                                                  |
| **Query DSL**              | `RunQueryDsl` (diesel-async)                      | Same `RunQueryDsl` — identical API                      |
| **Docker image size**      | Smaller (no C deps)                               | Slightly larger (needs `libpq-dev`)                     |

### When to use which?

**Choose `AsyncPgConnection`** if:

- You want a fully pure-Rust stack with no system C dependencies
- You need pipelining for high-throughput, concurrent query patterns
- You're okay with the one-time TLS setup complexity

**Choose `SyncConnectionWrapper<PgConnection>`** if:

- You want the simplest possible TLS setup (just a connection string)
- You're already linking `libpq` (e.g., for diesel CLI migrations)
- Pipelining is not a concern (pooling already gives you parallel connections)
- You value `libpq`'s battle-tested reliability

For most web applications, the `spawn_blocking` overhead of the sync wrapper is negligible
compared to actual database query latency. Pipelining only matters when you run many
concurrent queries on a **single** connection, which connection pooling already mitigates
by giving each request its own connection.

---

## Appendix: `sync-connection-wrapper` vs `async-connection-wrapper` Features

These are two separate diesel-async features that solve **opposite** problems:

| Feature                    | Direction    | You have → You need                             | Typical use case                                            |
| -------------------------- | ------------ | ----------------------------------------------- | ----------------------------------------------------------- |
| `sync-connection-wrapper`  | sync → async | Sync `PgConnection` → `AsyncConnection`         | Use diesel's sync `PgConnection` with async pooling/runtime |
| `async-connection-wrapper` | async → sync | `AsyncPgConnection` → sync `diesel::Connection` | Run diesel's sync migration runner in an async app          |

### `async-connection-wrapper`

Wraps an async connection so it looks sync. Useful for running diesel's migration runner
(which is sync) at app startup:

```rust
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;

let mut wrapper: AsyncConnectionWrapper<AsyncPgConnection> =
    AsyncConnectionWrapper::establish(database_url)?;

// Now you can use sync diesel_migrations runner
diesel_migrations::run_pending_migrations(&mut wrapper)?;
```

### `sync-connection-wrapper`

Wraps a sync connection so it implements `AsyncConnection`. This is the strategy discussed
in this document for easy TLS support:

```rust
use diesel_async::sync_connection_wrapper::SyncConnectionWrapper;
use diesel::PgConnection;

let async_conn: SyncConnectionWrapper<PgConnection> =
    SyncConnectionWrapper::establish(database_url).await?;

// Now use with diesel-async's RunQueryDsl, deadpool, etc.
```

---

## Appendix: Do You Need Explicit Dependencies?

When using the `AsyncPgConnection` TLS approach, you might wonder: "diesel-async already
depends on `tokio-postgres`, so do I really need to add it to my `Cargo.toml`?"

**Yes.** In Rust 2021+ edition (and certainly 2024 edition), transitive dependencies are
**not** accessible in your code without being listed in your own `[dependencies]`.

| Crate                   | Is it a transitive dep of diesel-async? | Do you need it explicitly? | Why?                                          |
| ----------------------- | --------------------------------------- | -------------------------- | --------------------------------------------- |
| `tokio-postgres`        | ✅ Yes (via `postgres` feature)         | ✅ Yes                     | You call `tokio_postgres::connect()` directly |
| `tokio-postgres-rustls` | ❌ No                                   | ✅ Yes                     | Completely external crate                     |
| `rustls`                | ❌ No                                   | ✅ Yes                     | You construct `rustls::ClientConfig` directly |
| `webpki-roots`          | ❌ No                                   | ✅ Yes                     | You use `TLS_SERVER_ROOTS` directly           |

Cargo will unify versions with the transitive dependency tree automatically — no duplicates.

With the `SyncConnectionWrapper` approach, **none of these are needed**.
