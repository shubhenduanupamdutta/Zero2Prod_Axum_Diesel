# Addendum for Chapter 4: Telemetry

---

// Intro as it is.

---

## 4.1 Unknown Unknowns

---

We have a few tests. Tests are good, they make us more confident in our software, in its correctness.
Nonetheless, a test suite is not proof of the correctness of our application. We would have to explore significantly different approaches to prove that something is correct (e.g. [formal methods](https://z2p.io/f6u)).

At runtime we will encounter scenarios that we have not tested for or even thought about when designing the application in the first place.

I can point at a few blind spots based on the work we have done so far and my past experiences:

- what happens if we lose connection to the database? Does `diesel_async::deadpool::Pool` try to automatically recover or will all database interactions fail from that point onwards until we restart the application?
- what happens if an attacker tries to pass malicious payloads in the body of the `POST /subscriptions` request (i.e. extremely large payloads, attempts to perform [SQL injection](http://en.wikipedia.org/wiki/SQL_injection), etc.)?

These are often referred to as **known unknowns**: shortcomings that we are aware of and we have not yet managed to investigate or we have deemed to be not relevant enough to spend time on.

Given enough time and effort, we could get rid of most known unknowns.

Unfortunately there are issues that we have not seen before and we are not expecting, **unknown unknowns**.

Sometimes experience is enough to transform an unknown unknown into a known unknown: if you had never worked with a database before you might have not thought about what happens when we lose connection; once you have seen it happen once, it becomes a familiar failure mode to look out for.

More often than not, unknown unknowns are peculiar failure modes of the specific system we are working on.

They are problems at the crossroads between our software components, the underlying operating systems, the hardware we are using, our development process peculiarities and that huge source of randomness known as "the outside world".

They might emerge when:

- the system is pushed outside of its usual operating conditions (e.g. an unusual spike of traffic);
- multiple components experience failures at the same time (e.g. a SQL transaction is left hanging while the database is going through a [master-replica failover](https://z2p.io/f6l));
- a change is introduced that moves the system equilibrium (e.g. tuning a retry policy);
- no changes have been introduced for a long time (e.g. applications have not been restarted for weeks and you start to see all sorts of memory leaks);
- etc.

All these scenarios share one key similarity: they are often impossible to reproduce outside of the live environment.

What can we do to prepare ourselves to deal with an outage or a bug caused by an unknown unknown?

---

## 4.2 Observability

---

// Everything as it is

---

## 4.3 Logging

---

// Intro as it is

### 4.3.1 The `log` crate

// Everything as it is

### 4.3.2 Logging in `axum`

Since `axum` supports `tower` middleware, we can use the `tower-http::trace::TraceLayer` to log incoming requests and outgoing responses.

For this we need to add `tower-http` as a dependency:

```toml
[dependencies]
tower-http = { version = "0.6.8", features = ["trace"] }
tracing = "0.1.44"
```

Then we can add the `TraceLayer` to our application:

```rust
//! src/startup.rs
use axum::{
    Router,
    routing::{get, post},
    serve::Serve,
};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

use crate::{
    DbPool,
    routes::{health_check, subscribe},
};

pub fn run(
    listener: TcpListener,
    pool: DbPool,
) -> Result<Serve<TcpListener, Router, Router>, std::io::Error> {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(pool)
        // Middlewares are added to the app using the `layer` method. Here, we are adding a tracing
        // layer that will log incoming requests and outgoing responses.
        .layer(TraceLayer::new_for_http());
    let server = axum::serve(listener, app);
    Ok(server)
}
```

We can now launch the application using cargo run and fire a quick request with curl `http://127.0.0.1:8000/health_check -v`.
The request comes back with a 200 but… nothing happens on the terminal we used to launch our application.
No logs. Nothing. Blank screen.

### 4.3.3 The Facade Pattern

We said that instrumentation is a local decision.

There is instead a global decision that applications are uniquely positioned to make: what are we supposed to do with all those log records?

Should we append them to a file? Should we print them to the terminal? Should we send them to a remote system over HTTP (e.g. [ElasticSearch](https://z2p.io/fvf))?

The `log` crate leverages the [facade pattern](https://z2p.io/fv2) to handle this duality.

It gives you the tools you need to emit log records, but it does not prescribe how those log records should be processed. It provides, instead, a `Log` trait:

```rust
//! From `log`'s source code - src/lib.rs
/// A trait encapsulating the operations required of a logger.
pub trait Log: Sync + Send {
    /// Determines if a log message with the specified metadata would be
    /// logged.
    ///
    /// This is used by the `log_enabled!` macro to allow callers to avoid
    /// expensive computation of log message arguments if the message would be
    /// discarded anyway.
    fn enabled(&self, metadata: &Metadata) -> bool;

    /// Logs the `Record`.
    ///
    /// Note that `enabled` is *not* necessarily called before this method.
    /// Implementations of `log` should perform all necessary filtering
    /// internally.
    fn log(&self, record: &Record);

    /// Flushes any buffered records.
    fn flush(&self);
}
```

At the beginning of your `main` function you can call the [`set_logger` function](https://z2p.io/fv4) and pass an implementation of the `Log` trait: every time a log record is emitted `Log::log` will be called on the logger you provided, therefore making it possible to perform whatever form of processing of log records you deem necessary.

If you do not call `set_logger`, then all log records will simply be discarded. Exactly what happened to our application.

Let's initialise our logger this time.

There are a few `Log` implementations available on [crates.io](https://crates.io/) — the most popular options are listed in the documentation of `log` itself.

We will use [`env_logger`](https://z2p.io/fvx) — it works nicely if, as in our case, the main goal is printing all log records to the terminal.

Let's add it as a dependency with

```toml
#! Cargo.toml
# [...]

[dependencies]
env_logger = "0.11.10"
# [...]
```

`env_logger::Logger` prints log records to the terminal, using the following format:

```sh
[<timestamp> <level> <module path>] <log message>
```

There is a problem though, by default `TraceLayer` doesn't emit `log` compatible records but instead uses the `tracing` crate. This means that, even if we initialise our logger, we won't see any log record emitted by `TraceLayer`.
For this we can enable feature 'log' in `tracing` dependency, which will make it possible to forward `tracing` events to the `log` compatible subscribers like `env_logger`.

```toml
[dependencies]
# [...]
tracing = { version = "0.1.44", features = ["log"] }
```

It looks at the `RUST_LOG` environment variable to determine what logs should be printed and what logs should be filtered out.

`RUST_LOG=debug cargo run`, for example, will surface all logs at debug-level or higher emitted by our application or the crates we are using. `RUST_LOG=zero2prod`, instead, would filter out all records emitted by our dependencies.

Let's modify our `main.rs` file as required:

```rust
// [...]
use env_logger::Env;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // `init` does call `set_logger`, so this is all we need to do.
    // We are falling back to printing all logs at info-level or above
    // if the RUST_LOG environment variable has not been set.
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    // [...]
}
```

Let's try to launch the application again using `RUST_LOG=trace,tower_http=trace cargo run` (equivalent to `RUST_LOG=info cargo run` given our defaulting logic). You should see startup log records appear on your terminal — axum itself emits fewer startup messages than some other frameworks, but any middleware built on `tower-http` (such as `TraceLayer`) will begin producing output once requests arrive.

If we make a request with `curl http://127.0.0.1:8000/health_check` you should see a log record emitted by the `TraceLayer` middleware we added a few paragraphs ago.

Logs are also an awesome tool to explore how the software we are using works.

`tower_http=trace` is needed to see the trace-level logs emitted by `TraceLayer` — without it, only info-level and above logs from `tower_http` would be printed.

```sh
[2026-04-19T13:40:05Z TRACE axum::serve] connection 127.0.0.1:53536 accepted
[2026-04-19T13:40:05Z DEBUG tower_http::trace::make_span] request; method=GET uri=/health_check version=HTTP/1.1
[2026-04-19T13:40:05Z TRACE tracing::span::active] -> request;
[2026-04-19T13:40:05Z DEBUG tower_http::trace::on_request] started processing request
[2026-04-19T13:40:05Z TRACE tracing::span::active] <- request;
[2026-04-19T13:40:05Z TRACE tracing::span::active] -> request;
[2026-04-19T13:40:05Z DEBUG tower_http::trace::on_response] finished processing request latency=0 ms status=200
[2026-04-19T13:40:05Z TRACE tracing::span::active] <- request;
[2026-04-19T13:40:05Z TRACE tracing::span] -- request;
[2026-04-19T13:40:05Z TRACE tracing::span] -- request;
```
