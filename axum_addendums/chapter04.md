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

---

## 4.4 Instrumenting POST /subscriptions

---

Let's use what we learned about `log` to instrument our handler for `POST /subscriptions` requests. It currently looks like this:

```rust
//! src/routes/subscriptions.rs
// [...]

pub async fn subscribe(State(pool): State<DbPool>, Form(form): Form<FormData>) -> StatusCode {
    // [...]

    match diesel::insert_into(subscriptions::table)
        .values(&new_subscriber)
        .execute(&mut conn)
        .await
    {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            // Using `eprintln!` to capture information about the error
            // in case things don't work out as expected.
            eprintln!("Failed to execute query: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        },
    }
}
```

Let's add `log` as a dependency:

```toml
#! Cargo.toml
# [...]
[dependencies]
log = "0.4.29"
```

What should we capture in log records?

### 4.4.1 Interactions With External Systems

Let's start with a tried-and-tested rule of thumb: any interaction with external systems over the network should be closely monitored. We might experience networking issues, the database might be unavailable, queries might get slower over time as the `subscriptions` table gets longer, etc.

Let's add two log records: one before query execution starts and one immediately after its completion.

```rust
//! src/routes/subscriptions.rs
// [...]

pub async fn subscribe(/* */) -> StatusCode {
    // [...]

    log::info!("Saving new subscriber details in the database");
    match diesel::insert_into(subscriptions::table)
        .values(&new_subscriber)
        .execute(&mut conn)
        .await
    {
        Ok(_) => {
            log::info!("New subscriber details have been saved");
            StatusCode::OK
        },
        Err(e) => {
            eprintln!("Failed to execute query: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
```

As it stands, we would only be emitting a log record when the query succeeds. To capture failures we need to convert that `eprintln!` statement into an error-level log:

```rust
//! src/routes/subscriptions.rs
// [...]

pub async fn subscribe(/* */) -> StatusCode {
    log::info!("Saving new subscriber details in the database");
    match diesel::insert_into(subscriptions::table)
        .values(&new_subscriber)
        .execute(&mut conn)
        .await
    {
        Ok(_) => {
            log::info!("New subscriber details have been saved");
            StatusCode::OK
        },
        Err(e) => {
            log::error!("Failed to execute query: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
```

Much better — we have that query somewhat covered now.

Pay attention to a small but crucial detail: we are using `{:?}`, the `std::fmt::Debug` format, to capture the query error.

Operators are the main audience of logs — we should extract as much information as possible about whatever malfunction occurred to ease troubleshooting. `Debug` gives us that raw view, while `std::fmt::Display` (`{}`) will return a nicer error message that is more suitable to be shown directly to our end users.

### 4.4.2 Think Like A User

What else should we capture?

Previously we stated that

> We will happily settle for an application that is sufficiently observable to enable us to deliver the level of service we promised to our users.

What does this mean _in practice_?

We need to change our reference system.

Forget, for a second, that we are the authors of this piece of software.

Put yourself in the shoes of one of your users, a person landing on your website that is interested in the content you publish and wants to subscribe to your newsletter.

What does a failure look like for them?

The story might play out like this:

> Hey!
>
> I tried subscribing to your newsletter using my main email address, thomas_mann@hotmail.com, but the website failed with a weird error. Any chance you could look into what happened?
>
> Best,
> Tom
>
> P.S. Keep it up, your blog rocks!

Tom landed on our website and received "a weird error" when he pressed the Submit button.

Our application is _sufficiently observable_ if we can triage the issue from the breadcrumbs of information he has provided us — i.e. the email address he entered.

Can we do it?

Let's, first of all, confirm the issue: is Tom registered as a subscriber?

We can connect to the database and run a quick query to double-check that there is no record with `thomas_mann@hotmail.com` as email in our `subscriptions` table.

The issue is confirmed. What now?

None of our logs include the subscriber email address, so we cannot search for it. Dead end.

We could ask Tom to provide additional information: all our log records have a timestamp, maybe if he remembers around what time he tried to subscribe we can dig something out?

This is a clear indication that our current logs are not good enough.

Let's improve them:

```rust
//! src/routes/subscriptions.rs
// [...]

pub async fn subscribe(/* */) -> StatusCode {
    // We are using the same interpolation syntax of `println`/`print` here!
    log::info!(
        "Adding '{}' '{}' as a new subscriber.",
        form.email,
        form.name
    );
    log::info!("Saving new subscriber details in the database");
    match diesel::insert_into(subscriptions::table)
        .values(&new_subscriber)
        .execute(&mut conn)
        .await
    {
        Ok(_) => {
            log::info!("New subscriber details have been saved");
            StatusCode::OK
        },
        Err(e) => {
            log::error!("Failed to execute query: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
```

Much better — we now have a log line that is capturing both name and email.

Is it enough to troubleshoot Tom's issue?

### 4.4.3 Logs Must Be Easy To Correlate

If we had a single copy of our web server running at any point in time and that copy was only capable of handling a single request at a time, we might imagine logs showing up in our terminal more or less like this:

```sh
# First request
[.. INFO zero2prod] Adding 'thomas_mann@hotmail.com' 'Tom' as a new subscriber
[.. INFO zero2prod] Saving new subscriber details in the database
[.. INFO zero2prod] New subscriber details have been saved
[.. INFO zero2prod] .. "POST /subscriptions HTTP/1.1" 200 ..

# Second request
[.. INFO zero2prod] Adding 's_erikson@malazan.io' 'Steven' as a new subscriber
[.. ERROR zero2prod] Failed to execute query: connection error with the database
[.. ERROR zero2prod] .. "POST /subscriptions HTTP/1.1" 500 ..
```

You can clearly see where a single request begins, what happened while we tried to fulfill it, what we returned as a response, where the next request begins, etc.

It is easy to follow.

But this is not what it looks like when you are handling multiple requests concurrently:

```sh
[.. INFO zero2prod] Receiving request for POST /subscriptions
[.. INFO zero2prod] Receiving request for POST /subscriptions
[.. INFO zero2prod] Adding 'thomas_mann@hotmail.com' 'Tom' as a new subscriber
[.. INFO zero2prod] Adding 's_erikson@malazan.io' 'Steven' as a new subscriber
[.. INFO zero2prod] Saving new subscriber details in the database
[.. ERROR zero2prod] Failed to execute query: connection error with the database
[.. ERROR zero2prod] .. "POST /subscriptions HTTP/1.1" 500 ..
[.. INFO zero2prod] Saving new subscriber details in the database
[.. INFO zero2prod] New subscriber details have been saved
[.. INFO zero2prod] .. "POST /subscriptions HTTP/1.1" 200 ..
```

What details did we fail to save though? `thomas_mann@hotmail.com` or `s_erikson@malazan.io`?

Impossible to say from the logs.

We need a way to correlate all logs related to the same request.

This is usually achieved using a **request id** (also known as **correlation id**): when we start to process an incoming request we generate a random identifier (e.g. a UUID) which is then associated to all logs concerning the fulfilling of that specific request.

Let's add one to our handler:

```rust
//! src/routes/subscriptions.rs
// [...]

pub async fn subscribe(/* */) -> StatusCode {
    // Let's generate a random unique identifier
    let request_id = Uuid::new_v4();
    log::info!(
        "request_id {} - Adding '{}' '{}' as a new subscriber.",
        request_id,
        form.email,
        form.name
    );
    // [...]
    log::info!(
        "request_id {} - Saving new subscriber details in the database",
        request_id
    );
    match diesel::insert_into(subscriptions::table)
        .values(&new_subscriber)
        .execute(&mut conn)
        .await
    {
        Ok(_) => {
            log::info!(
                "request_id {} - New subscriber details have been saved",
                request_id
            );
            StatusCode::OK
        },
        Err(e) => {
            log::error!(
                "request_id {} - Failed to execute query: {:?}",
                request_id,
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
```

Logs for an incoming request will now look like this:

```sh
curl -i -X POST -d 'email=thomas_mann@hotmail.com&name=Tom' \
    http://127.0.0.1:8000/subscriptions

[2026-04-19T14:05:55Z INFO  zero2prod::routes::subscriptions] request_id 4200e77b-c0da-4a67-b493-5f934d4f624d - Adding 'thomas_mann@hotmail.com' 'Tom' as a new subscriber.
[2026-04-19T14:05:55Z INFO  zero2prod::routes::subscriptions] request_id 4200e77b-c0da-4a67-b493-5f934d4f624d - Saving new subscriber details in the database
[2026-04-19T14:05:55Z INFO  zero2prod::routes::subscriptions] request_id 4200e77b-c0da-4a67-b493-5f934d4f624d - New subscriber details have been saved successfully.
```

We can now search for `thomas_mann@hotmail.com` in our logs, find the first record, grab the `request_id` and then pull down all the other log records associated with that request.

Well, almost all the logs: `request_id` is created in our `subscribe` handler, therefore `tower-http`'s `TraceLayer` middleware is completely unaware of it.

That means that we will not know what status code our application has returned to the user when they tried to subscribe to our newsletter.

What should we do?

We could bite the bullet, remove `tower-http`'s `TraceLayer`, write a middleware to generate a random request identifier for every incoming request and then write our own logging middleware that is aware of the identifier and includes it in all log lines.

Could it work? Yes.

Should we do it? Probably not.

---

## 4.5 Structured Logging

---

To ensure that `request_id` is included in all log records we would have to:

- rewrite all upstream components in the request processing pipeline (e.g. axum's `TraceLayer`);
- change the signature of all downstream functions we are calling from the `subscribe` handler; if they are emitting a log statement, they need to include the `request_id`, which therefore needs to be passed down as an argument.

What about log records emitted by the crates we are importing into our project? Should we rewrite those as well?

It is clear that this approach cannot scale.

Let's take a step back: what does our code look like?

We have an over-arching task (an HTTP request), which is broken down in a set of sub-tasks (e.g. parse input, make a query, etc.), which might in turn be broken down in smaller sub-routines recursively.
Each of those units of work has a duration (i.e. a beginning and an end).
Each of those units of work has a context associated to it (e.g. name and email of a new subscriber, `request_id`) that is naturally shared by all its sub-units of work.

No doubt we are struggling: log statements are isolated events happening at a defined moment in time that we are stubbornly trying to use to represent a tree-like processing pipeline.

Logs are the wrong abstraction.

What should we use then?

### 4.5.1 The 'tracing' crate

// Intro as it is

### 4.5.2 Migrating from 'log' to 'tracing'

There is only one way to find out - let's convert our `subscribe` handler to use `tracing` instead of `log` for instrumentation. Let's add `tracing` to our dependencies.

```toml
#! Cargo.toml
[dependencies]
tracing = { version = "0.1.44", features = ["log"] }
# [...]
```

The first migration step is as straight-forward as it gets: search and replace all occurrences of the `log` string in our function with tracing.

```rust
//! src/routes/subscriptions.rs
// [...]

pub async fn subscribe(State(pool): State<DbPool>, Form(form): Form<FormData>) -> StatusCode {
    let request_id = Uuid::new_v4();
    tracing::info!(
        "request_id {} - Adding '{}' '{}' as a new subscriber.",
        request_id,
        form.email,
        form.name
    );

    let new_subscriber = InsertSubscription {
        id: Uuid::new_v4(),
        email: form.email,
        name: form.name,
        subscribed_at: Utc::now(),
    };

    let mut conn = pool
        .get()
        .await
        .expect("Failed to get a connection from the pool.");

    tracing::info!(
        "request_id {} - Saving new subscriber details in the database",
        request_id
    );
    match diesel::insert_into(subscriptions::table)
        .values(&new_subscriber)
        .execute(&mut conn)
        .await
    {
        Ok(_) => {
            tracing::info!(
                "request_id {} - New subscriber details have been saved successfully.",
                request_id
            );
            StatusCode::OK
        },
        Err(e) => {
            tracing::error!(
                "request_id {} - Failed to execute query: {:?}",
                request_id,
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        },
    }
}
```

That's it.
If you run the application and fire a `POST /subscriptions` request, you will see _exactly the same logs_ in your console. Identical.
Pretty cool, isn't it?

This works thanks to [`tracing`'s log feature flag](https://z2p.io/fvm), which we have enabled in `Cargo.toml`. It ensures that every time an event or a span are created using `tracing`'s macros a corresponding log even is emitted, allowing `log`'s loggers to pick up on it (`env_logger` in our case).

### 4.5.3 `tracing`'s Span

We can now start to leverage `tracing`'s [Span](https://z2p.io/fv3) to better capture the structure of our program.
We want to create a span that represents the whole HTTP request.

```rust
//! src/routes/subscriptions.rs
// [...]

pub async fn subscribe(/* */) -> StatusCode {
    let request_id = Uuid::new_v4();
    // Spans, like logs, have an associated level
    // `info_span` creates a span at the info-level
    let request_span = tracing::info_span!(
        "Adding a new subscriber.",
        %request_id,
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    );
    // Using `enter` in an async function is a recipe for disaster!
    // Bear with me for now, but don't do this at home.
    // See the following section on `Instrumenting Futures`
    let _request_span_guard = request_span.enter();

    // [...]

    // `_request_span_guard` is dropped at the end of `subscribe`
    // That's when we "exit" the span
}
```

There is a lot going on here - let's break it down.

We are using the `info_span!` macro to create a new span and attach some values to its context: `request_id`, `form.email` and `form.name`.

We are not using string interpolation anymore: `tracing` allows us to associate _structured_ information to our spans as a collection of key-value pairs. We can explicitly name them (e.g. `subscriber_email` for `form.email`) or implicitly use the variable name as key (e.g. the isolated `request_id` is equivalent to `request_id = request_id`).

Notice that we prefixed all of them with a `%` symbol: we are telling `tracing` to use their `Display` implementation for logging purposes. You can find more details on the other available options in [their documentation](https://z2p.io/fvq).

`info_span` returns the newly created span, but we have to explicitly step into it using the `.enter()` method to activate it.

`.enter()` returns an instance of [`Entered`](https://z2p.io/fv5), a guard: as long as the guard variable is not dropped all downstream spans and log events will be registered as _children_ of the entered span. This is a [typical Rust pattern](https://z2p.io/fvw), often referred to as **R**resource **A**cquisition **I**s **I**nitialization (RAII): the compiler keeps track of the lifetime of all variables and when they go out of scope it inserts a call to their destructor, [`Drop::drop`](https://z2p.io/fv7).

The default implementation of the `Drop` trait simply takes care of releasing the resources owned by that variable. We can, though, specify a custom `Drop` implementation to perform other cleanup operations on drop - e.g. exiting from a span when the `Entered` guard gets dropped:

```rust
//! `tracing`'s source code

impl<'a> Drop for Entered<'a> {
    #[inline]
    fn drop(&mut self) {
        // Dropping the guard exits the span.
        //
        // Running this behaviour on drop rather than with an explicit function
        // call means that spans may still be exited when unwinding.
        if let Some(inner) = self.span.inner.as_ref() {
            inner.subscriber.exit(&inner.id);
        }

        if_log_enabled! {{
            if let Some(ref meta) = self.span.meta {
                self.span.log(
                    ACTIVITY_LOG_TARGET,
                    log::Level::Trace,
                    format_args!("<- {}", meta.name())
                );
            }
        }}

    }
}
```

Inspecting the source code of your dependencies can often expose some gold nuggets - we just found out that if the `log` feature flag is enabled `tracing` will emit a trace-level log when a span exits.

Let's give it a go immediately:

```sh
RUST_LOG=trace cargo run
```

```sh
[... INFO  zero2prod::routes::subscriptions] Adding a new subscriber.; request_id=ec51f2b1-87d5-4740-b85f-08eb5e64a6cb subscriber_email=thomas_mann@hotmail.com subscriber_name=Tom
[... TRACE tracing::span::active] -> Adding a new subscriber.;
[... INFO  zero2prod::routes::subscriptions] request_id ec51f2b1-87d5-4740-b85f-08eb5e64a6cb - Adding 'thomas_mann@hotmail.com' 'Tom' as a new subscriber.
[... INFO  zero2prod::routes::subscriptions] request_id ec51f2b1.. - Saving new subscriber details in the database
[... INFO  zero2prod::routes::subscriptions] request_id ec51f2b1.. - New subscriber details have been saved successfully.
[... TRACE tracing::span::active] <- Adding a new subscriber.;
[... TRACE tracing::span] -- Adding a new subscriber.;
[... DEBUG tower_http::trace::on_response] finished processing request latency=44 ms status=200
```

Notice how all the information we captured in the span's context is reported in the emitted log line.

We can closely follow the lifetime of our span using the emitted logs:

- `Adding a new subscriber` is logged when the span is created;
- We enter the span (`->`);
- We execute the `INSERT` query;
- We exit the span (`<-`);
- We finally close the span (`--`).

Wait, what is the difference between exiting and closing a span?

Glad you asked!

You can enter (and exit) a span multiple times. Closing, instead, is final: it happens when the span itself is dropped.

This comes pretty handy when you have a unit of work that can be paused and then resumed - e.g. an asynchronous task!

### 4.5.4 Instrumenting Futures

Let's use our database query as an example.

The executor might have to [poll its future](https://z2p.io/fve) more than once to drive it to completion - while that future is idle, we are going to make progress on other futures.

This can clearly cause issues: how do we make sure we don't mix their respective spans?

The best way would be to closely mimic the future's lifecycle: we should enter into the span associated to our future every time it is polled by the executor and exit every time it gets parked.

That's where [`Instrument`](https://z2p.io/fv9) comes into the picture. It is an extension trait for futures. `Instrument::instrument` does exactly what we want: enters the span we pass as argument every time `self`, the future, is polled; it exits the span every time the future is parked.

Let's try it out on our query:

```rust
//! src/routes/subscriptions.rs
use tracing::Instrument;
// [...]

pub async fn subscribe(State(pool): State<DbPool>, Form(form): Form<FormData>) -> impl IntoResponse {
    let request_id = Uuid::new_v4();
    let request_span = tracing::info_span!(
        "Adding a new subscriber.",
        %request_id,
        subscriber_email = %form.email,
        subscriber_name = %form.name
    );
    let _request_span_guard = request_span.enter();

    // We do not call `.enter` on query_span!
    // `.instrument` takes care of it at the right moments
    // in the query future lifetime
    let query_span = tracing::info_span!(
        "Saving new subscriber details in the database"
    );

    let mut conn = pool
        .get()
        .await
        .expect("Failed to get a connection from the pool.");

    match diesel::insert_into(subscriptions::table)
        .values(&new_subscriber)
        // First we attach the instrumentation, then we `.await` it
        .execute(&mut conn)
        .instrument(query_span)
        .await
    {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            // Yes, this error log falls outside of `query_span`
            // We'll rectify it later, pinky swear!
            tracing::error!("Failed to execute query: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
```

If we launch the application again with `RUST_LOG=trace` and try a `POST /subscriptions` request we will see logs that look somewhat similar to these:

```sh
[.. INFO  zero2prod::routes::subscriptions] Adding a new subscriber.; request_id=f323cb66-ebad-43b2-9c00-8d6981c54788 subscriber_email=thomas_mann@hotmail.com subscriber_name=Tom
[.. TRACE tracing::span::active] -> Adding a new subscriber.;
[.. INFO  tracing::span] Saving new subscriber details in the database;
[.. TRACE zero2prod] -> Saving new subscriber details in the database
[.. TRACE zero2prod] <- Saving new subscriber details in the database
[.. TRACE zero2prod] -> Saving new subscriber details in the database
[.. TRACE zero2prod] <- Saving new subscriber details in the database
[.. TRACE zero2prod] -> Saving new subscriber details in the database
[.. TRACE zero2prod] <- Saving new subscriber details in the database
[.. TRACE zero2prod] -> Saving new subscriber details in the database
[.. TRACE zero2prod] -> Saving new subscriber details in the database
[.. TRACE zero2prod] <- Saving new subscriber details in the database
[.. TRACE zero2prod] -- Saving new subscriber details in the database
[.. TRACE zero2prod] <- Adding a new subscriber.
[.. TRACE zero2prod] -- Adding a new subscriber.
```

We can clearly see how many times the query future has been polled by the executor before completing. How cool is that!?

### 4.5.5 `tracing`'s Subscriber

We embarked in this migration from `log` to `tracing` because we needed a better abstraction to instrument our code effectively. We wanted, in particular, to attach `request_id` to all logs associated to the same incoming HTTP request.

Although I promised `tracing` was going to solve our problem, look at those logs: `request_id` is only printed on the very first log statement where we attach it explicitly to the span context.

Why is that?

Well, we haven't completed our migration yet.

Although we moved all our instrumentation code from `log` to `tracing` we are still using `env_logger` to process everything!

```rust
//! src/main.rs
//! [...]

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    env_logger::from_env(Env::default().default_filter_or("info")).init();
    // [...]
}
```

`env_logger`'s logger implements `log`'s `Log` trait - it knows nothing about the rich structure exposed by `tracing`'s `Span`!

`tracing`'s compatibility with `log` was great to get off the ground, but it is now time to replace `env_logger` with a `tracing`-native solution.

The `tracing` crate follows the same facade pattern used by `log` - you can freely use its macros to instrument your code, but applications are in charge to spell out how that span telemetry data should be processed.

[`Subscriber`](https://z2p.io/fvr) is the `tracing` counterpart of `log`'s `Log`: an implementation of the `Subscriber` trait exposes a variety of methods to manage every stage of the lifecycle of a `Span` - creation, enter/exit, closure, etc.

```rust
//! `tracing`'s source code
pub trait Subscriber: 'static {
    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id;
    fn event(&self, event: &Event<'_>);
    fn enter(&self, span: &span::Id);
    fn exit(&self, span: &span::Id);
    fn clone_span(&self, id: &span::Id) -> span::Id;
    // [...]
}
```

The quality of `tracing`'s documentation is breath-taking - I _strongly_ invite you to have a look for yourself at [`Subscriber`'s docs](https://z2p.io/fvr) to properly understand what each of those methods does.

### 4.5.6 `tracing-subscriber`

// Everything as it is

### 4.5.7 `tracing-bunyan-formatter`

We'd like to put together a subscriber that has feature-parity with the good old `env_logger`.

We will get there by combining three layers:

- [`tracing_subscriber::filter::EnvFilter`](https://z2p.io/fvp) discards spans based on their log levels and their origins, just as we did in `env_logger` via the `RUST_LOG` environment variable;
- [`tracing_bunyan_formatter::JsonStorageLayer`](https://z2p.io/fvl) processes spans data and stores the associated metadata in an easy-to-consume JSON format for downstream layers. It does, in particular, propagate context from parent spans to their children;
- [`tracing_bunyan_formatter::BunyanFormattingLayer`](https://z2p.io/fvk) builds on top of `JsonStorageLayer` and outputs log records in [bunyan](https://z2p.io/fvs)-compatible JSON format.

We are using `tracing-bunyan-formatter` instead of the formatting layer provided by `tracing-subscriber` because the latter does not implement metadata inheritance: it would therefore fail to meet our requirements.

Let's add `tracing_bunyan_formatter` to our dependencies:

```toml
[dependencies]
# ...
tracing-bunyan-formatter = "0.3.10"
```

We can now tie everything together in our main function:

```rust
//! src/main.rs
//! [...]
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
        // We removed the `env_logger` line we had before!
        // We are falling back to printing all spans at info-level or above
        // if the RUST_LOG environment variable has not been set.
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"));
        let formatting_layer = BunyanFormattingLayer::new(
            "zero2prod".into(),
            // Output the formatted spans to stdout.
            std::io::stdout,
        );
        // The `with` method is provided by `SubscriberExt`, an extension
        // trait for `Subscriber` exposed by `tracing_subscriber`
        let subscriber = Registry::default()
            .with(env_filter)
            .with(JsonStorageLayer)
            .with(formatting_layer);
        // `set_global_default` can be used by applications to specify
        // what subscriber should be used to process spans.
        set_global_default(subscriber).expect("Failed to set subscriber");
        // [...]
}
```

If you launch the application with `cargo run` and fire a request you'll see these logs (pretty-printed here to be easier on the eye):

```json
{
    "v": 0,
    "name": "zero2prod",
    "msg": "[ADDING A NEW SUBSCRIBER. - START]",
    "level": 30,
    "hostname": "****",
    "pid": 427708,
    "time": "2026-04-25T16:36:10.884813883Z",
    "target": "zero2prod::routes::subscriptions",
    "line": 31,
    "file": "src/routes/subscriptions.rs",
    "request_id": "3beff053-6e2a-4a7d-9a7b-bf7b99a54f9d",
    "subscriber_name": "Tom",
    "subscriber_email": "thomas_mann@hotmail.com"
}
{
    "v": 0,
    "name": "zero2prod",
    "msg": "[SAVING NEW SUBSCRIBER DETAILS IN THE DATABASE - START]",
    "level": 30,
    "hostname": "****",
    "pid": 427708,
    "time": "2026-04-25T16:36:10.884893662Z",
    "target": "zero2prod::routes::subscriptions",
    "line": 43,
    "file": "src/routes/subscriptions.rs",
    "request_id": "3beff053-6e2a-4a7d-9a7b-bf7b99a54f9d",
    "subscriber_name": "Tom",
    "subscriber_email": "thomas_mann@hotmail.com"
}
{
    "v": 0,
    "name": "zero2prod",
    "msg": "[SAVING NEW SUBSCRIBER DETAILS IN THE DATABASE - END]",
    "level": 30,
    "hostname": "****",
    "pid": 427708,
    "time": "2026-04-25T16:36:10.932037882Z",
    "target": "zero2prod::routes::subscriptions",
    "line": 43,
    "file": "src/routes/subscriptions.rs",
    "request_id": "3beff053-6e2a-4a7d-9a7b-bf7b99a54f9d",
    "elapsed_milliseconds": 3,
    "subscriber_name": "Tom",
    "subscriber_email": "thomas_mann@hotmail.com"
}
{
    "v": 0,
    "name": "zero2prod",
    "msg": "[ADDING A NEW SUBSCRIBER. - END]",
    "level": 30,
    "hostname": "****",
    "pid": 427708,
    "time": "2026-04-25T16:36:10.932130925Z",
    "target": "zero2prod::routes::subscriptions",
    "line": 31,
    "file": "src/routes/subscriptions.rs",
    "request_id": "3beff053-6e2a-4a7d-9a7b-bf7b99a54f9d",
    "elapsed_milliseconds": 47,
    "subscriber_name": "Tom",
    "subscriber_email": "thomas_mann@hotmail.com"
}
```

We made it: everything we attached to the original context has been propagated to all its sub-spans.

`tracing-bunyan-formatter` also provides duration out-of-the-box: every time a span is closed a JSON message is printed to the console with an `elapsed_milliseconds` property attached to it.

The JSON format is extremely friendly when it comes to searching: an engine like ElasticSearch can easily ingest all these records, infer a schema and index the `request_id`, `name` and `email` fields. It unlocks the full power of a querying engine to sift through our logs!

This is exponentially better than we had before: to perform complex searches we would have had to use custom-built regexes, therefore limiting considerably the range of questions that we could easily ask to our logs.

### 4.5.8 `tracing-log`

If you take a closer look you will realise we lost something along the way: our terminal is only showing logs that were directly emitted by our application. What happened to the log records emitted by dependencies that still use the `log` facade?

`tracing`'s `log` feature flag ensures that a log record is emitted every time a `tracing` event happens, allowing `log`'s loggers to pick them up.

The opposite does not hold true: `log` does not emit `tracing` events out of the box and does not provide a feature flag to enable this behaviour.

If we want it, we need to explicitly register a logger implementation to redirect logs to our `tracing` subscriber for processing. `tracing-subscriber` has our back here: if we enable the `tracing-log` feature flag it'll redirect all `log`'s logs to our subscriber.

We can use [`LogTracer`](https://z2p.io/fvg), provided by the [`tracing-log`](https://z2p.io/fvj) crate.

```toml
#! Cargo.toml
# [...]
[dependencies]
tracing-log = "0.2"
# [...]
```

Let's edit our `main` as required:

```rust
//! src/main.rs
//! [...]
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Redirect all `log`'s events to our subscriber
    LogTracer::init().expect("Failed to set logger");
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=debug"));
    let formatting_layer = BunyanFormattingLayer::new(
        "zero2prod".into(),
        std::io::stdout,
    );
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    set_global_default(subscriber).expect("Failed to set subscriber");
    // [...]
}
```

All logs emitted through the `log` facade should once again be available in our console.

### 4.5.9 Removing Unused Dependencies

// Everything as it is

### 4.5.10 Cleaning Up Initialization

We relentlessly pushed forward to improve the observability posture of our application.
Let's now take a step back and look at the code we wrote to see if we can improve in any meaningful way.

Let's start from our `main` function:

```rust
//! src/main.rs
//! use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{AsyncDieselConnectionManager, deadpool::Pool},
};
use tokio::net::TcpListener;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};
use zero2prod::{configuration::get_configuration, startup::run};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    LogTracer::init().expect("Failed to set logger");

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=debug"));
    let formatting_layer = BunyanFormattingLayer::new("zero2prod".into(), std::io::stdout);

    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    set_global_default(subscriber).expect("Failed to get subscriber");

    // Panic if we can't read configuration
    let configuration = get_configuration().expect("Failed to read configuration.");
    let db_config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(
        configuration.database.connection_string(),
    );
    let pool = Pool::builder(db_config)
        .build()
        .expect("Failed to create connection pool.");

    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address).await?;
    run(listener, pool)?.await
}
```

There is a lot going on in that `main` function right now.
Let's break it down a bit:

```rust
//! src/main.rs
use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{AsyncDieselConnectionManager, deadpool::Pool},
};
use tokio::net::TcpListener;
use tracing::{Subscriber, subscriber::set_global_default};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};
use zero2prod::{configuration::get_configuration, startup::run};

/// Compose multiple layers into a `tracing`'s subscriber.
///
/// # Implementation Notes
///
/// We are using `impl Subscriber` as return type to avoid having to
/// spell out the actual type of the returned subscriber, which is
/// indeed quite complex.
/// We need to explicitly call out that the returned subscriber is
/// `Send` and `Sync` to make it possible to pass it to `init_subscriber`
/// later on.
pub fn get_subscriber(name: String, env_filter: String) -> impl Subscriber + Send + Sync {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let formatting_layer = BunyanFormattingLayer::new(name, std::io::stdout);

    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

/// Register a subscriber as global default to process span data.
///
/// It should only be called once!
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info,http_tower=debug".into());
    init_subscriber(subscriber);

    // Panic if we can't read configuration
    let configuration = get_configuration().expect("Failed to read configuration.");
    let db_config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(
        configuration.database.connection_string(),
    );
    let pool = Pool::builder(db_config)
        .build()
        .expect("Failed to create connection pool.");

    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address).await?;
    run(listener, pool)?.await
}
```

We can now move `get_subscriber` and `init_subscriber` to a module within our `zero2prod` library, `telemetry`.

```rust
//! src/lib.rs
use diesel_async::{AsyncPgConnection, pooled_connection::deadpool};

pub mod configuration;
pub mod routes;
pub mod schema;
pub mod startup;
pub mod telemetry;

pub type DbPool = deadpool::Pool<AsyncPgConnection>;
```

```rust
//! src/telemetry.rs
use tracing::{Subscriber, subscriber::set_global_default};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};

pub fn get_subscriber(name: String, env_filter: String) -> impl Subscriber + Send + Sync {
    // [...]
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}
```

```rust
//! src/main.rs
use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{AsyncDieselConnectionManager, deadpool::Pool},
};
use tokio::net::TcpListener;
use zero2prod::{
    configuration::get_configuration,
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};


#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info,http_tower=debug".into());
    init_subscriber(subscriber);

    // [...]
}
```

Awesome.

### 4.5.11 Logs For Integration Tests

We are not just cleaning up for aesthetic/readability reasons - we are moving those two functions to the `zero2prod` library to make them available to our test suite!

As a rule of thumb, everything we use in our application should be reflected in our integration tests.
Structured logging, in particular, can significantly speed up our debugging when an integration test fails: we might not have to attach a debugger, more often than not the logs can tell us where something went wrong. It is also a good benchmark: if you cannot debug it from logs, imagine how difficult would it be to debug in production!

Let's change our `spawn_app` helper function to take care of initialising our `tracing` stack:

```rust
//! tests/health_check.rs
use diesel::{prelude::*, sql_query};
use diesel_async::{
    AsyncConnection,
    AsyncPgConnection,
    RunQueryDsl,
    pooled_connection::{AsyncDieselConnectionManager, deadpool::Pool},
};
use diesel_migrations::{FileBasedMigrations, MigrationHarness};
use tokio::net::TcpListener;
use uuid::Uuid;
use zero2prod::{
    DbPool,
    configuration::{DatabaseSettings, get_configuration},
    schema::subscriptions,
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};

pub struct TestApp {
    pub address: String,
    pub db_pool: DbPool,
}

async fn spawn_app() -> TestApp {
    let subscriber = get_subscriber("test".into(), "debug,tower_http=trace".into());
    init_subscriber(subscriber);

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to address");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();
    let pool = configure_database(&configuration.database).await;

    let server = run(listener, pool.clone()).expect("Failed to start server");
    let _server_handle = tokio::spawn(server.into_future());

    TestApp { address, db_pool: pool }
}

// [...]
```

If you try to run `cargo test` you will be greeted by one success and a long series of test failures:

```sh
failures:

---- subscribe_returns_a_200_for_valid_form_data stdout ----

thread 'subscribe_returns_a_200_for_valid_form_data' (21512) panicked at src/telemetry.rs:18:23:
Failed to set logger: SetLoggerError(())
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- health_check_works stdout ----

thread 'health_check_works' (21511) panicked at src/telemetry.rs:18:23:
Failed to set logger: SetLoggerError(())


failures:
    health_check_works
    subscribe_returns_a_200_for_valid_form_data

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
```

`init_subscriber` should only be called once, but it is being invoked by all our tests.
We can use [`std::sync::LazyLock`](https://z2p.io/fwa) to fix the issue:

```rust
//! tests/health_check.rs
use std::sync::LazyLock;
// [...]

// Ensures that the `tracing` stack is only initialized once using `LazyLock`
static TRACING: LazyLock<()> = LazyLock::new(|| {
    let subscriber = get_subscriber("test".into(), "debug,tower_http=trace".into());
    init_subscriber(subscriber);
});

pub struct TestApp {
    pub address: String,
    pub db_pool: DbPool,
}

async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    LazyLock::force(&TRACING);

    // [...]
}

// [...]
```

`cargo test` is green again.

The output, though, is very noisy: we have several log lines coming out of each test case.
We want our tracing instrumentation to be exercised in every test, but we do not want to look at those logs _every time_ we run our test suite.

`cargo test` solves the very same problem for `println!` and `print!` statements. By default, it swallows everything that is printed to console. You can explicitly opt in to look at those print statements using `cargo test -- --nocapture`.

We need an equivalent strategy for `our tracing` instrumentation.
Let's add a new parameter to `get_subscriber` to allow customisation of what sink logs should be written to:

```rust
//! src/telemetry.rs
// [...]
use tracing_subscriber::{EnvFilter, Registry, fmt::MakeWriter, layer::SubscriberExt};

pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    // This "weird" syntax is a higher-ranked trait bound (HRTB)
    // It basically means that Sink implements the `MakeWriter`
    // trait for all choices of the lifetime parameter `'a`
    // Check out https://doc.rust-lang.org/nomicon/hrtb.html
    // for more details.
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    // [...]
    let formatting_layer = BunyanFormattingLayer::new(name, sink);
    // [...]
}
```

We can then adjust our `main` function to use `stdout`:

```rust
//! src/main.rs
// [...]

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber(
        "zero2prod".into(),
        "info,tower_http=debug".into(),
        std::io::stdout,
    );
    init_subscriber(subscriber);

    // [...]
}
```

In our test suite we will choose the sink dynamically according to an environment variable, `TEST_LOG`. If `TEST_LOG` is set, we use `std::io::stdout`.
If `TEST_LOG` is not set, we send all logs into the void using `std::io::sink`.
Our own home-made version of the `--nocapture` flag.

```rust
//! tests/health_check.rs
//! [...]

// Ensures that the `tracing` stack is only initialized once using `LazyLock`
static TRACING: LazyLock<()> = LazyLock::new(|| {
    let default_filter_level = "info,tower_http=trace".to_string();
    let subscriber_name = "test".to_string();
    // We cannot assign the output of `get_subscriber` to a variable based on the
    // value TEST_LOG` because the sink is part of the type returned by
    // `get_subscriber`, therefore they are not the same type. We could work around
    // it, but this is the most straight-forward way of moving forward.
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

// [...]
```

When you want to see all logs coming out of a certain test case to debug it you can run

```sh
# We are using the `bunyan` CLI to prettify the outputted logs
# The original `bunyan` requires NPM, but you can install a Rust-port with
# `cargo install bunyan`
TEST_LOG=true cargo test health_check_works | bunyan
```

and sift through the output to understand what is going on.
Neat, isn't it?

### 4.5.12 Cleaning Up Instrumentation Code - `tracing::instrument`

We refactored our initialisation logic. Let's have a look at our instrumentation code now.
Time to bring `subscribe` back once again.

```rust
//! src/routes/subscriptions.rs
// [...]

pub async fn subscribe(State(pool): State<DbPool>, Form(form): Form<FormData>) -> StatusCode {
    let request_id = Uuid::new_v4();

    let request_span = tracing::info_span!(
        "Adding a new subscriber.",
        %request_id,
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    );

    let _request_span_guard = request_span.enter();

    // We do not call `.enter` on query_span!
    // `.instrument` takes care of it at the right moments
    // in the query lifetime
    let query_span = tracing::info_span!("Saving new subscriber details in the database");

    let new_subscriber = InsertSubscription {
        id: Uuid::new_v4(),
        email: form.email,
        name: form.name,
        subscribed_at: Utc::now(),
    };

    let mut conn = pool
        .get()
        .await
        .expect("Failed to get a connection from the pool.");

    match diesel::insert_into(subscriptions::table)
        .values(&new_subscriber)
        .execute(&mut conn)
        .instrument(query_span)
        .await
    {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            // Yes, this error log falls outside of `query_span`
            // We'll rectify it later, pinky swear!
            tracing::error!("Failed to execute query: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        },
    }
}
```

It is fair to say logging has added some noise to our `subscribe` function.
Let's see if we can cut it down a bit.

We will start with `request_span`: we'd like all operations within `subscribe` to happen within the context of `request_span`.
In other words, we'd like to _wrap_ the `subscribe` function in a span.

This requirement is fairly common: extracting each sub-task in its own function is a common way to structure routines to improve readability and make it easier to write tests; therefore we will often want to _attach_ a span to a function declaration.

`tracing` caters for this specific use case with its `tracing::instrument` procedural macro. Let's see it in action:

```rust
//! src/routes/subscriptions.rs
// [...]


#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        request_id = %Uuid::new_v4(),
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(State(pool): State<DbPool>, Form(form): Form<FormData>) -> StatusCode {
    let query_span = tracing::info_span!("Saving new subscriber details in the database");

    let new_subscriber = InsertSubscription {
        id: Uuid::new_v4(),
        email: form.email,
        name: form.name,
        subscribed_at: Utc::now(),
    };

    let mut conn = pool
        .get()
        .await
        .expect("Failed to get a connection from the pool.");

    match diesel::insert_into(subscriptions::table)
        .values(&new_subscriber)
        .execute(&mut conn)
        .instrument(query_span)
        .await
    {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            tracing::error!("Failed to execute query: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        },
    }
}
```

`#[tracing::instrument]` creates a span at the beginning of the function invocation and automatically attaches all arguments passed to the function to the context of the span. In our case, that would include the axum extractors passed to `subscribe`.

Often function arguments won't be displayable on log records or we'd like to specify more explicitly what should be captured and how it should be recorded. We can explicitly tell `tracing` to ignore them using the `skip` directive.

`name` can be used to specify the message associated with the function span. If omitted, it defaults to the function name.

We can also enrich the span's context using the `fields` directive. It leverages the same syntax we have already seen for the `info_span!` macro.

The result is quite nice: all instrumentation concerns are visually separated from execution concerns. The former are dealt with in a procedural macro that decorates the function declaration, while the function body focuses on the actual business logic.

It is important to point out that `tracing::instrument` also takes care of using `Instrument::instrument` when it is applied to an asynchronous function.

Let's extract the database write in its own function and use `tracing::instrument` to get rid of `query_span` and the call to `.instrument`:

```rust
//! src/routes/subscriptions.rs
// [...]

#[derive(Insertable)]
#[diesel(table_name = subscriptions)]
pub struct InsertSubscription<'a> {
    id: Uuid,
    email: &'a str,
    name: &'a str,
    subscribed_at: DateTime<Utc>,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        request_id = %Uuid::new_v4(),
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(State(pool): State<DbPool>, Form(form): Form<FormData>) -> StatusCode {
    let new_subscriber = InsertSubscription {
        id: Uuid::new_v4(),
        email: &form.email,
        name: &form.name,
        subscribed_at: Utc::now(),
    };

    match insert_subscriber(&pool, &new_subscriber).await {
        Ok(_) => StatusCode::OK,
        Err(e) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(form, pool)
)]
pub async fn insert_subscriber(
    pool: &DbPool,
    subscriber: &InsertSubscription<'_>,
) -> Result<(), diesel::result::Error> {
    let mut conn = pool
        .get()
        .await
        .expect("Failed to get a connection from the pool.");

    diesel::insert_into(subscriptions::table)
        .values(subscriber)
        .execute(&mut conn)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        // Using the `?` operator to return early
        // if the function failed, returning a `diesel::result::Error` to the caller
        // We will talk about error handling in depth later.
        })?;
    Ok(())
}
```

The error event now falls within the query span and we have a better separation of concerns:

- `insert_subscriber` takes care of the database logic and it has no awareness of the surrounding web framework, i.e. we are not passing axum's `Form` or `State` wrappers as input types;
- `subscribe` orchestrates the work to be done by calling the required routines and translates their outcome into the proper response according to the rules and conventions of the HTTP protocol.

I must confess my unbounded love for `tracing::instrument`: it significantly lowers the effort required to instrument your code.
It pushes you into the **pit of success**: the right thing to do is the easiest thing to do.

### 4.5.13 Protect Your Secrets - `secrecy`

There is actually one element of `#[tracing::instrument]` that I am not fond of: it automatically attaches all arguments passed to the function to the context of the span - you have to **opt-out** of logging function inputs (via `skip`) rather than **opt-in**.

You do not want secrets (e.g. a password) or personal identifiable information (e.g. the billing address of an end user) in your logs.
Opt-out is a dangerous default - every time you add a new input to a function using `#[tracing::instrument]` you need to ask yourself: is it safe to log this? Should I skip it?
Give it enough time and somebody will forget - you now have a security incident to deal with.
You can prevent this scenario by introducing a wrapper type that explicitly marks which fields are considered
to be sensitive - `secrecy::Secret`.

```toml
#! Cargo.toml
# [...]
[dependencies]
secrecy = { version = "0.10.3", features = ["serde"] }
# [...]
```

Let's checkout its definition:

```rust
/// Secret string type.
///
/// This is a type alias for [`SecretBox<str>`] which supports some helpful trait impls.
///
/// Notably it has a [`From<String>`] impl which is the preferred method for construction.
pub type SecretString = SecretBox<str>;

impl From<String> for SecretString {
    fn from(s: String) -> Self {
        Self::from(s.into_boxed_str())
    }
}

impl From<&str> for SecretString {
    fn from(s: &str) -> Self {
        Self::from(String::from(s))
    }
}

impl Clone for SecretString {
    fn clone(&self) -> Self {
        SecretBox {
            inner_secret: self.inner_secret.clone(),
        }
    }
}

impl Default for SecretString {
    fn default() -> Self {
        String::default().into()
    }
}

/// Wrapper type for values that contains secrets, which attempts to limit
/// accidental exposure and ensure secrets are wiped from memory when dropped.
/// (e.g. passwords, cryptographic keys, access tokens or other credentials)
///
/// Access to the secret inner value occurs through the [`ExposeSecret`]
/// or [`ExposeSecretMut`] traits, which provide methods for accessing the inner secret value.
pub struct SecretBox<S: Zeroize + ?Sized> {
    inner_secret: Box<S>,
}
```

Memory wiping, provided by the `Zeroize` trait, is a nice-to-have.
The key property we are looking for is `Secret`’s masked Debug implementation: `println!("{:?}", my_secret_string)` outputs Secret([REDACTED String]) instead of the actual secret value. This is exactly what we need to prevent accidental leakage of sensitive material via `#[tracing::instrument]` or other logging statements.

There is an additional upside to an explicit wrapper type: it serves as documentation for new developers who are being introduced to the codebase. It nails down what is considered sensitive in your domain/according to the relevant regulation.

The only secret value we need to worry about, right now, is the database password. Let's wrap it up.

```rust
//! src/configuration.rs
use secrecy::SecretString;
use serde::Deserialize;

// [...]

#[derive(Deserialize)]
pub struct DatabaseSettings {
    // [...]
    pub password: SecretString,
}
// [...]
```

`Secret` does not interfere with deserialization - it implements `serde::Deserialize` by delegating to the wrapped type, provided that the `serde` feature is enabled.

The compiler is not happy:

```sh
error[E0277]: `SecretBox<str>` doesn't implement `std::fmt::Display`
  --> src/configuration.rs:23:28
   |
22 |             "postgres://{}:{}@{}:{}/{}",
   |                            -- required by this formatting parameter
23 |             self.username, self.password, self.host, self.port, self.database_name
   |                            ^^^^^^^^^^^^^ `SecretBox<str>` cannot be formatted with the default formatter
   |
```

That is a feature, not a bug - `secrecy::Secret` does not implement `Display`, therefore we need to explicitly allow the exposure of the wrapped secret. The compiler error is a useful prompt to notice that the entire database connection string should be marked as `Secret` as well, given that it embeds the database password.

```rust
//! src/configuration.rs
use secrecy::ExposeSecret;
// [...]

impl DatabaseSettings {
    pub fn connection_string(&self) -> SecretString {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name
        )
        .into()
    }
}


pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    // Initialize our configuration reader
    let settings = config::Config::builder()
        .add_source(
            config::File::new("configuration.yaml", config::FileFormat::Yaml)
        )
        .build()?;

    // Try to convert the configuration values it read into our settings type
    settings.try_deserialize::<Settings>()
}
```

When wiring up the application in `main`, the same principle applies: expose the connection string only when building the Diesel connection pool.

```rust
//! src/main.rs
use secrecy::ExposeSecret;
// [...]

[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // [...]
    let db_config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(
        configuration.database.connection_string().expose_secret(),
    );
    // [...]
}
```

Likewise, in the test helpers used to create or configure the database, we only expose the secret at the point where Diesel needs the actual connection string value.

```rust
//! tests/health_check.rs
use secrecy::ExposeSecret;
// [...]

pub async fn configure_database(config: &DatabaseSettings) -> DbPool {
    // Create Database
    let maintenance_settings = DatabaseSettings {
        database_name: "postgres".to_string(),
        username: "postgres".to_string(),
        password: "password".into(),
        ..config.clone()
    };

    let mut connection =
        AsyncPgConnection::establish(maintenance_settings.connection_string().expose_secret())
            .await
            .expect("Failed to connect to Postgres");

    // [...]
    {
        let mut connection = PgConnection::establish(config.connection_string().expose_secret())
            .expect("Failed to connect to Postgres");
        connection
            .run_pending_migrations(
                FileBasedMigrations::find_migrations_directory()
                    .expect("Failed to find migration directory."),
            )
            .expect("Failed to run database migrations.");
    }


    // Create the connection pool and return it
    let connection_pool = AsyncDieselConnectionManager::<AsyncPgConnection>::new(
        config.connection_string().expose_secret(),
    );
    Pool::builder(connection_pool)
        .build()
        .expect("Failed to create connection pool.")
}
```

This is it for the time being - going forward we will make sure to wrap sensitive values into `Secret` as soon as they are introduced.

### 4.5.14 Request Id

We have one last job to do: ensure all logs for a particular request, in particular the record with the returned status code, are enriched with a `request_id` property. How?

In our Axum application we are already using a tracing-aware middleware for HTTP logs: `tower_http::trace::TraceLayer`. That means we do not need to replace the logging middleware with another crate just to get structured tracing output.

What we are missing is request id propagation.

`TraceLayer` will happily emit structured logs for request start, response status code and latency, but it does not generate or attach a request identifier on its own. To get that behaviour we need to compose it with the request-id utilities provided by `tower-http`.

Let's add the `request-id` feature for `tower-http` and add `tower` to our dependencies:

```toml
#! Cargo.toml
# [...]
[dependencies]
tower = { version = "0.5.3", features = ["tracing"] }
tower-http = { version = "0.6.8", features = ["trace", "request-id"] }
# [...]
```

The request-id middleware is in charge of:

- generating a unique request identifier when the incoming request does not already provide one;
- storing that identifier in the request extensions/headers so downstream middleware and handlers can access it;
- optionally propagating the identifier back to the client via a response header.

That still leaves one piece to wire up: we want the logs emitted by `TraceLayer`, including the one carrying the returned status code, to contain `request_id` as a structured field.

To make that happen we configure `TraceLayer` with a custom span factory. When the span for the HTTP request is created, we read the request identifier inserted by the request-id layer and record it as part of the span context.

Once `request_id` is attached to the request span, all events emitted by `TraceLayer` within that span inherit it automatically. This includes the response log carrying the final status code.

The way to do it ergonomically is to create a apply middleware function that applies configured middleware to to application router in `telemetry.rs` and apply it in `startup.rs` when building the application.

```rust
//! src/telemetry.rs
use axum::{
    Router,
    body::Body,
    http::{HeaderName, Request},
};
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

// [...]

const REQUEST_ID_HEADER: &str = "x-request-id";

// [...]

pub fn apply_tracing_with_req_id_middleware(router: Router) -> Router {
    let x_request_id = HeaderName::from_static(REQUEST_ID_HEADER);
    let middleware = ServiceBuilder::new()
        .layer(SetRequestIdLayer::new(
            x_request_id.clone(),
            MakeRequestUuid,
        ))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<Body>| {
                let request_id = request.headers().get(REQUEST_ID_HEADER);
                match request_id {
                    Some(request_id) => {
                        info_span!(
                            "http_request",
                            request_id = ?request_id,
                        )
                    },
                    None => {
                        tracing::error!("Could not extract request_id");
                        info_span!("http_request")
                    },
                }
            }),
        )
        .layer(PropagateRequestIdLayer::new(x_request_id));
    router.layer(middleware)
}
```

```rust
//! src/startup.rs
// [...]
use crate::telemetry::apply_tracing_with_req_id_middleware;

pub fn run(
    listener: TcpListener,
    pool: DbPool,
) -> Result<Serve<TcpListener, Router, Router>, std::io::Error> {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(pool);

    let app = apply_tracing_with_req_id_middleware(app);
    let server = axum::serve(listener, app);
    Ok(server)
}
```

Middleware ordering matters here: the request-id layer must run before the tracing layer tries to create its span, otherwise there will be no identifier to attach.

If you also want clients and reverse proxies to see the identifier, you can propagate the same header back on the response, which is what `PropagateRequestIdLayer` does. This is optional, but it can be useful for debugging and correlation purposes on the client side.

From this point onward, a single `request_id` can be used to correlate our application logs with the structured request/response logs emitted by `tower-http`.

There is complicated string in the `EnvFilter` configuration in `main` that you may have noticed. `format!("{}=info,tower_http=trace,axum::rejection=trace", env!("CARGO_PKG_NAME"))` is a way to set the log level for our application crate to `info` while keeping the default log level for `tower-http` and `axum::rejection` at `trace`. This is necessary to see the structured request/response logs emitted by `TraceLayer`.

If you launch the application and fire a request you should see a `request_id` on all logs as well as `request_path` and a few other useful bits of information.

We are almost done - there is one outstanding issue we need to take care of.
let's take a closer look at the emitted log records for a `POST / subscriptions` request:

```sh
{
    "v": 0,
    "name": "zero2prod",
    "msg": "[ADDING A NEW SUBSCRIBER - END]",
    "request_id": "12d6aa4a-027f-40a6-a73b-06acbce7f30e",
    ...
}
{
    "v": 0,
    "name": "zero2prod",
    "msg": "[HTTP_REQUEST - EVENT] finished processing request",
    "request_id": "c0419855-d54e-49d7-bafb-a0c9937481ef",
    ...
}
```

We have two different `request_id`s for the same request.

The bug can be traced back to the instrumentation on our `subscribe` handler: we are still generating a `request_id` at the function level, which overrides the one attached to the request span by the request-id middleware.

```rust
//! src/routes/subscriptions.rs
// [...]

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        request_id = %Uuid::new_v4(),
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(State(pool): State<DbPool>, Form(form): Form<FormData>) -> StatusCode {
    // [...]
}

// [...]
```

:et's get rid of the `request_id` field in the instrumentation and let the request-id middleware take care of it.

```rust
//! src/routes/subscriptions.rs
// [...]

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(State(pool): State<DbPool>, Form(form): Form<FormData>) -> StatusCode {
    // [...]
}

// [...]
```

All good now - we have one consistent `request_id` for each endpoint of our application.

### 4.5.15 Leveraging The `tracing` Ecosystem

We covered a lot of what `tracing` has to offer - it has significantly improved the quality of the telemetry data we are collecting as well as the clarity of our instrumentation code.

At the same time, we have barely touched upon the richness of the whole `tracing` ecosystem when it comes to subscriber layers and integrations.

Just to mention a few more of those readily available:

- the spans emitted by our application and by `tower-http`'s `TraceLayer` can be wired into `tracing-opentelemetry` to ship telemetry data to an OpenTelemetry-compatible service (e.g. Jaeger or Honeycomb.io) for further analysis;
- `tracing-error` enriches our error types with a `SpanTrace` to ease troubleshooting.

It is not an exaggeration to state that `tracing` is a foundational crate in the Rust ecosystem. While `log` remains the minimum common denominator, `tracing` has become the backbone for structured diagnostics and observability in many Rust applications.

---

## 4.6 Summary

---

We started from a completely silent Axum application and we ended up with high-quality telemetry data. It is now time to take this newsletter API live!

In the next chapter we will build a basic deployment pipeline for our Rust project.
