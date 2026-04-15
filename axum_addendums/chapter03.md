# Addendum for Chapter 3

---

## 3.2 Choosing A Web Framework

---

What web framework should we use to write our Rust API?
You can find many competing options in the ecosystem (actix-web, axum, poem, tide, rocket, etc.). For this book, For this book, we will use axum.

axum is a web framework built by the Tokio team, making it a natural fit for the async Rust ecosystem. It has seen rapidly growing production adoption, benefits from a large and active community, and integrates seamlessly with the broader Tower middleware ecosystem via `tower` and `tower-http`; last but not least, it is built directly on top of tokio, therefore minimizing the likelihood of having to deal with incompatibilities and interop between different async runtimes.

Axum will therefore be our choice for Zero To Production.
Throughout this chapter and beyond, I suggest you keep a couple of extra browser tabs open:

- [axum's repository](https://github.com/tokio-rs/axum),
- [axum's documentation](https://docs.rs/axum/latest/axum/), and
- [axum's examples collection](https://github.com/tokio-rs/axum/tree/main/examples)

---

## 3.3 Our First Endpoint: A Basic Health Check

---

// Intro as it is

### 3.3.1 Wiring Up `axum`

Our starting point will be an _Hello World!_ application built with `axum`

```rust
use axum::{Router, extract::Path, routing::get};
use tokio::net::TcpListener;

async fn greet(name: Option<Path<String>>) -> impl IntoResponse {
    let name = name.map(|Path(n)| n).unwrap_or("World".into());
    format!("Hello, {}!", name)
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app = Router::new()
        .route("/", get(greet))
        .route("/{name}", get(greet));

    let listener = TcpListener::bind("127.0.0.1:8000").await?;
    axum::serve(listener, app).await
}
```

Let's paste it in our `main.rs` file.
A quick `cargo check`:

```sh
error[E0433]: failed to resolve: use of unresolved module or unlinked crate `axum`
 --> src/main.rs:1:5
  |
1 | use axum::{Router, extract::Path, routing::get};
  |     ^^^^ use of unresolved module or unlinked crate `axum`
  |
  = help: if you wanted to use a crate named `axum`, use `cargo add axum` to add it to your `Cargo.toml`

error[E0433]: failed to resolve: use of unresolved module or unlinked crate `tokio`
 --> src/main.rs:2:5
  |
2 | use tokio::net::TcpListener;
  |     ^^^^^ use of unresolved module or unlinked crate `tokio`
  |
  = help: if you wanted to use a crate named `tokio`, use `cargo add tokio` to add it to your `Cargo.toml`

error[E0432]: unresolved import `axum`
 --> src/main.rs:1:5
  |
1 | use axum::{Router, extract::Path, routing::get};
  |     ^^^^ use of unresolved module or unlinked crate `axum`
  |
  = help: if you wanted to use a crate named `axum`, use `cargo add axum` to add it to your `Cargo.toml`

error[E0433]: failed to resolve: use of unresolved module or unlinked crate `tokio`
 --> src/main.rs:9:3
  |
9 | #[tokio::main]
  |   ^^^^^ use of unresolved module or unlinked crate `tokio`

error[E0433]: failed to resolve: use of unresolved module or unlinked crate `axum`
  --> src/main.rs:16:5
   |
16 |     axum::serve(listener, app).await
   |     ^^^^ use of unresolved module or unlinked crate `axum`
   |
   = help: if you wanted to use a crate named `axum`, use `cargo add axum` to add it to your `Cargo.toml`

error[E0752]: `main` function is not allowed to be `async`
  --> src/main.rs:10:1
   |
10 | async fn main() -> Result<(), std::io::Error> {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `main` function is not allowed to be `async`

Some errors have detailed explanations: E0432, E0433, E0752.
For more information about an error, try `rustc --explain E0432`.
error: could not compile `Zero2Prod_Axum_Diesel` (bin "Zero2Prod_Axum_Diesel") due to 6 previous errors
```

We have not added `axum` and `tokio` to our dependencies yet, therefore the compiler cannot resolve what we imported.
We can either fix the situation manually, by adding

```toml
#! Cargo.toml
# [...]

[dependencies]
axum = "0.8.8"
tokio = { version = "1.51.0", features = ["macros", "rt-multi-thread"] }
```

under `[dependencies]` in our `Cargo.toml`, or we can simply run the following commands:

```sh
cargo add axum
cargo add tokio --features macros,rt-multi-thread
```

If you run `cargo check` again, you should see that the errors are gone, and the compiler is happy with our code.
You can now launch the application with `cargo run` and perform a quick manual test:

```sh
curl http://127.0.0.1:8000
```

```sh
Hello, World!
```

Cool, it's **alive!**
You can gracefully shut down the web server by pressing `CTRL+C` in the terminal where it's running.

### 3.3.2 Anatomy Of An `axum` Application

Let's go back now to have a closer look at what we have just copy-pasted in our `main.rs` file.

```rust
//! src/main.rs
// [...]

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app = Router::new()
        .route("/", get(greet))
        .route("/{name}", get(greet));

    let listener = TcpListener::bind("127.0.0.1:8000").await?;
    axum::serve(listener, app).await
}
```

#### 3.3.2.1 Server - `TcpListener` + `axum::serve`

In axum, transport-level concerns are handled by two separate pieced from the standard async ecosystem - the `TcpListener` from the `tokio` crate, and the `axum::serve` function.

- `tokio::net::TcpListener` binds to a TCP socket and listens for incoming connections (e.g. `127.0.0.1:8000` in our case)

- `axum::serve` accepts connections from the listener and drives the application

`axum::serve` takes care of the connection lifecycle — accepting new connections, feeding requests to the`Router`, and writing responses back — while `TcpListener`owns the socket-level concerns. For TLS, Unix domain sockets, or connection limits, you would reach for`tower`middleware or configure the listener before passing it to`axum::serve`.

3.3.2.2 Application — `Router`

`Router` is where all your application logic lives: routing, middleware, request handlers, etc. Its job is to take an incoming request and produce a response.

```rust
let app = Router::new()
    .route("/", get(greet))
    .route("/{name}", get(greet));
```

Just like actix-web's `App`, `Router` is a practical example of the **builder pattern**: `Router::new()` gives you a clean slate, and you add behaviour incrementally by chaining method calls. We will explore more of `Router`'s API surface on a need-to-know basis throughout the book.

#### 3.3.2.3 Endpoint - `route()`

The `.route()` method is the primary way to register a new endpoint on a `Router`. It takes two parameters:

- **`path`** — a string, possibly templated (e.g. `"/{name}"`) to accommodate dynamic path segments
- **`method_router`** — an instance of `MethodRouter`, which pairs an HTTP method with a handler

```rust
.route("/", get(greet))
.route("/{name}", get(greet));
```

`get(greet)`is a shorthand that creates a`MethodRouter`that only matches HTTP`GET`requests and dispatches them to the`greet`
handler. axum provides equivalent helpers for all HTTP methods:`post()`, `put()`, `delete()`, `patch()`, etc.

When a new request arrives, `Router` walks its registered routes until it finds one whose path template and HTTP method both match, then passes the request to the associated handler.

You can start to picture what happens when a new request comes in: `Router` iterates over all registered endpoints until it finds a matching one (both path template and guards are satisfied) and passes over the request object to the handler.
This is not 100% accurate but it is a good enough mental model for the time being.
What does a handler look like instead? What is its function signature?
We only have one example at the moment, `greet`:

```rust
async fn greet(name: Option<Path<String>>) -> impl Responder {
[...]
}
```

`greet` is an async function whose arguments are **extractors** — types that know how to pull data out of an incoming request. Here, `Option<Path<String>>` attempts to extract a path segment called `name`; if the route has no such segment (e.g. `/`), it gracefully returns `None`.

The return type must implement axum's `IntoResponse` trait. A type implements the `IntoResponse` trait if it can be converted into an HTTP response. This trait is implemented for variety of common types out of the box like `String`, `&str`, `Json<T>`, `(StatusCode, String)` tuples, `Response`, and more.
In our case, `String` implements `IntoResponse`, so we can return a string directly from our handler, and axum will take care of converting it into a proper HTTP response with a 200 status code and the string as the body.

Do all handlers need to share the same signature? No. axum accepts a wide variety of function signatures, as long as all arguments implement `FromRequest` or `FromRequestParts`. We will revisit this in detail as we add more endpoints.

#### 3.3.2.4 Runtime - `tokio`

We drilled down from the whole `axum::serve` to a `Route`. Let’s look again at the whole main function:

```rust
//! src/main.rs
// [...]

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app = Router::new()
        .route("/", get(greet))
        .route("/{name}", get(greet));

    let listener = TcpListener::bind("127.0.0.1:8000").await?;
    axum::serve(listener, app).await
}
```

What is `#[tokio::main]` doing here? Well, let's remove it and see what happens! `cargo check` screams at us with these errors:

```sh
error[E0752]: `main` function is not allowed to be `async`
  --> src/main.rs:10:1
   |
10 | async fn main() -> Result<(), std::io::Error> {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `main` function is not allowed to be `async`

For more information about this error, try `rustc --explain E0752`.
error: could not compile `Zero2Prod_Axum_Diesel` (bin "Zero2Prod_Axum_Diesel") due to 1 previous error
```

We need main to be asynchronous because `axum::serve` is an async function — but why can't `main` simply be `async`?

Asynchronous programming in Rust is built on top of the `Future` trait: a future stands for a value that may not be there yet. All futures expose a `poll` method which has to be called to allow the future to make progress and eventually resolve to its final value. You can think of Rust's futures as **lazy**: unless polled, there is no guarantee that they will execute to completion. This is often described as a **pull model**, compared to the push model adopted by other languages.

Rust's standard library, by design, does not include an asynchronous runtime: you are supposed to bring one into your project as a dependency — one more crate under `[dependencies]` in your `Cargo.toml`. This approach is extremely versatile: you are free to use any runtime that fits your use case, or even implement your own.

This explains why `main` cannot be an asynchronous function: **who would be in charge of calling `poll` on it?** There is no special configuration syntax that tells the Rust compiler that one of your dependencies is an asynchronous runtime, and there is not even a standardised definition of what a runtime is (e.g. no `Executor` trait in `std`).

You are therefore expected to launch your asynchronous runtime at the top of your `main` function and then use it to drive your futures to completion.

`#[tokio::main]` is a procedural macro that handles exactly this. It rewrites your `async fn main()` into a synchronous `fn main()` that boots the tokio runtime and uses it to block on the async body you wrote. In other words:

```rust
//! what you write
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // ...
}
```

gets expanded by the macro into roughly:

```rust
//! what the compiler sees
fn main() -> Result<(), std::io::Error> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            // ...
        })
}
```

`block_on` drives the future passed to it to completion, polling it whenever it is ready to make progress. This is the bridge between the synchronous world of `main` and the asynchronous world of `axum::serve` and `TcpListener::bind`.

Since axum is built by the tokio team and runs entirely on tokio, `#[tokio::main]` is the natural and idiomatic choice for bootstrapping an axum application.

### 3.3.3 Implementing the Health Check Handler

We have reviewed all the moving pieces in axum's Hello World! example: `TcpListener`, `axum::serve`, `Router`, `route`, and `tokio::main`.

We definitely know enough to modify the example to get our health check working as we expect: return a 200 OK response with no body when we receive a GET request at `/health_check`.

Let's look again at our starting point:

```rust
//! src/main.rs
use axum::{Router, extract::Path, response::IntoResponse, routing::get};
use tokio::net::TcpListener;

async fn greet(name: Option<Path<String>>) -> impl IntoResponse {
    let name = name.map(|Path(n)| n).unwrap_or("World".into());
    format!("Hello, {}!", name)
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app = Router::new()
        .route("/", get(greet))
        .route("/{name}", get(greet));

    let listener = TcpListener::bind("127.0.0.1:8000").await?;
    axum::serve(listener, app).await
}
```

First of all we need a request handler. Mimicking `greet` we can start with this signature:

```rust
async fn health_check() -> impl IntoResponse {
    todo!()
}
```

We said that `IntoResponse` is nothing more than a conversion trait into an HTTP response. Returning an instance of `StatusCode` directly should work then!

axum's `StatusCode` type implements `IntoResponse` directly. We can return `StatusCode::OK` to produce a 200 response with an empty body — no builder or extra method calls needed.

Gluing everything together:

```rust
use axum::http::StatusCode;

async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}
```

A quick `cargo check` confirms that our handler is not doing anything weird. Notice that, unlike actix-web. axum does not complain about the fact that we are not using any of the data bundled with the incoming HTTP request (e.g. headers, query parameters, etc.) — this is because axum handlers declare only the extractors they actually need, and in this case we don't need any.

The next step is handler registration — we need to add it to our `Router` via `.route()`:

```rust
Router::new()
    .route("/health_check", get(health_check))
```

Let's look at the full picture:

```rust
//! src/main.rs
use axum::{Router, http::StatusCode, response::IntoResponse, routing::get};
use tokio::net::TcpListener;


async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app = Router::new().route("/health_check", get(health_check));
    let listener = TcpListener::bind("127.0.0.1:8000").await?;
    axum::serve(listener, app).await
}

```

`cargo check` runs smoothly with no warnings.

What is left to do?

Well, a little test!

```sh
# Launch the application first in another terminal with `cargo run`
curl -v http://127.0.0.1:8000/health_check
```

```sh
*   Trying 127.0.0.1:8000...
* Connected to 127.0.0.1 (127.0.0.1) port 8000
> GET /health_check HTTP/1.1
> Host: 127.0.0.1:8000
> User-Agent: curl/8.5.0
> Accept: */*
>
< HTTP/1.1 200 OK
< content-length: 0
< date: Sat, 04 Apr 2026 13:14:13 GMT
<
* Connection #0 to host 127.0.0.1 left intact
```

Congrats, you have just implemented your first working axum endpoint!

---

## 3.4 Our First Integration Test

---

// Everything as it is

### 3.4.1 How Do You Test An Endpoint?

// Everything as it is

### 3.4.2 Where Should I Put My Tests?

// Everything as it is

### 3.4.3 Changing Our Project Structure For Easier Testing

// Intro as it is
// Only last bit of actual code in `main.rs` and `lib.rs` is changed

```rust
//! main.rs

use zero2prod::run;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    run().await
}
```

```rust
//! lib.rs
use axum::{Router, http::StatusCode, response::IntoResponse, routing::get};
use tokio::net::TcpListener;


async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

// We need to mark `run` as public.
// It is no longer a binary entrypoint, therefore we can mark it as async
// without having to use any proc-macro incantation.
pub async fn run() -> Result<(), std::io::Error> {
    let app = Router::new().route("/health_check", get(health_check));
    let listener = TcpListener::bind("127.0.0.1:8000").await?;
    axum::serve(listener, app).await
}
```

Alright, we are ready to write some juicy integration tests!

---

## 3.5 Implementing Our First Integration Test

---

Our spec for the health check endpoint was:

> When we receive a GET request for `/health_check` we return a 200 OK response with no body.

Let's translate that into a test, filling in as much of it as we can:

```rust
//! tests/health_check.rs
// `tokio::test` is the testing equivalent of `tokio::main`.
// It also spares you from having to specify the `#[test]` attribute.
//
// You can inspect what code gets generated using
// `cargo expand --test health_check` (<- name of the test file)
#[tokio::test]
async fn health_check_works() {
    // Arrange
    spawn_app().await.expect("Failed to spawn our app.");
    // We need to bring in `reqwest`
    // to perform HTTP requests against our application.
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get("http://127.0.0.1:8000/health_check")
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

// Launch our application in the background ~somehow~
async fn spawn_app() -> Result<(), std::io::Error> {
    todo!()
}
```

```toml
#! Cargo.toml
# [...]
# Dev dependencies are used exclusively when running tests or examples
# They do not get included in the final application binary!
[dev-dependencies]
reqwest = "0.13.2"
# [...]
```

Take a second to really look at this test case.

`spawn_app` is the only piece that will, reasonably, depend on our application code. Everything else is entirely decoupled from the underlying implementation details — if tomorrow we decide to ditch Rust and rewrite our application in Ruby on Rails we can still use the same test suite to check for regressions in our new stack as long as `spawn_app` gets replaced with the appropriate trigger (e.g. a bash command to launch the Rails app).

The test also covers the full range of properties we are interested to check:

- the health check is exposed at `/health_check`;
- the health check is behind a GET method;
- the health check always returns a 200;
- the health check's response has no body.

If this passes we are done.

The test as it is crashes before doing anything useful: we are missing `spawn_app`, the last piece of the integration testing puzzle.

Why don't we just call `run` in there? I.e.

```rust
//! tests/health_check.rs
// [...]
async fn spawn_app() -> Result<(), std::io::Error> {
    zero2prod::run().await
}
```

Let's try it out!

```sh
cargo test
```

```sh
Running target/debug/deps/health_check-fc74836458377166

running 1 test
test health_check_works ...
test health_check_works has been running for over 60 seconds
```

No matter how long you wait, test execution will never terminate. What is going on?

In `zero2prod::run` we invoke (and await) `axum::serve`. `axum::serve` starts listening on the address we specified indefinitely: it will handle incoming requests as they arrive, but it will never shut down or "complete" on its own.

This implies that `spawn_app` never returns and our test logic never gets executed.

We need to run our application as a background task.

`tokio::spawn` comes quite handy here: `tokio::spawn` takes a future and hands it over to the runtime for polling, without waiting for its completion; it therefore runs concurrently with downstream futures and tasks (e.g. our test logic).

Let's refactor `zero2prod::run` to return a future handle without awaiting it. In axum, `axum::serve(...)` returns a `Serve` future — we can wrap the whole server setup in a way that lets the caller decide when to `.await` it. The idiomatic approach is to return a `tokio::task::JoinHandle` from `spawn_app` directly, or to restructure `run` so it builds the server but leaves the `.await` to the caller.

The simplest approach mirrors the book: separate _building_ the server from _running_ it.

```rust
//! src/lib.rs
use axum::{Router, http::StatusCode, response::IntoResponse, routing::get, serve::Serve};
use tokio::net::TcpListener;


async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}


// Notice the different signature!
// We return a `Result` with `Serve<TcpListener, Router, Router>` as the success type, which is the type returned by
// `axum::serve` without awaiting it. This type is not a future but implements `IntoFuture`, so it can be awaited by
// the caller when they are ready to run the server.
// Also notice that run now takes in a `TcpListener` as an argument, instead of creating it internally. because this is
// `tokio::net::TcpListener` which can only be called with await after binding. But this function is not async now.
// This  also allows us to create a listener on a random port in our tests and pass it to `run`, avoiding port conflicts
// and allowing for more flexible testing setups.
pub fn run(listener: TcpListener) -> Result<Serve<TcpListener, Router, Router>, std::io::Error> {
    let app = Router::new().route("/health_check", get(health_check));
    let server = axum::serve(listener, app);
    Ok(server)
}
```

We need to amend our `main.rs` accordingly:

```rust
//! src/main.rs

use tokio::net::TcpListener;
use zero2prod::run;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Bubble up the `io::Error` if we failed to bind the address
    let listener = TcpListener::bind("127.0.0.1:8000").await?;
    // Now we can call `run` and await the server future it returns
    run(listener)?.await
}
```

A quick `cargo check` confirms that our code is still valid, and we can now adjust `spawn_app` to call `run` and spawn the server in the background:

```rust
//! tests/health_check.rs
// [...]
// No .await call on `spawn_app` itself, so it doesn't need to be async
// in the sense that it returns immediately after handing the server off
// to the tokio executor.
// We are also running tests, so it is not worth it to propagate errors:
// if we fail to perform the required setup we can just panic and crash
// all the things.
async fn spawn_app() {
    let listener = TcpListener::bind("127.0.0.1:8000")
        .await
        .expect("Failed to bind to address");
    let server = zero2prod::run(listener).expect("Failed to start server");
    // Launch the server as a background task, allowing the test logic to run concurrently
    // `tokio::spawn` returns a `JoinHandle` that we could use to await the server's completion if
    // we wanted to, but in this case we don't care about that since the server is supposed to run
    // indefinitely.
    // `tokio::spawn` only handles `Future` so we need to call `.into_future()` on the server to
    // convert it into a future that can be spawned.
    let _server_handle = tokio::spawn(server.into_future());
}
```

Quick adjustment to our test to accommodate the changes in `spawn_app`'s signature:

```rust
//! tests/health_check.rs
// [...]
#[tokio::test]
async fn health_check_works() {
    // no .expect
    spawn_app().await;
    // [...]
}
```

It's time, let's run that `cargo test` command!

```sh
cargo test
```

```sh
     Running tests/health_check.rs (target/debug/deps/health_check-65296ccb710e00e7)

running 1 test
test health_check_works ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

Yay! Our first integration test is green!

Give yourself a pat on the back on my behalf for the second major milestone in the span of a single chapter.

### 3.5.1 Polishing

// Everything as it is

#### 3.5.1.1 Clean Up

// Everything as it is

#### 3.5.1.2 Choosing A Random Port

`spawn_app` will always try to run our application on port 8000 - not ideal:

- if port 8000 is being used by another program on our machine (e.g. our own application!), tests will fail;
- if we try to run two or more tests in parallel only one of them will manage to bind the port, all others will fail.

We can do better: tests should run their background application on a random available port.
Our run implementation is already flexible since it is taking a `tokio::net::TcpListener` as an argument, so we just need to change the way we create the listener in `spawn_app`:

How do we find a random available port for our tests?
The operating system comes to the rescue: we will be using **port 0**.
Port 0 is a special-cased at the OS level: trying to bind port 0 will trigger an OS scan for available port which will then be bound to the application.

It is therefore enough to change `spawn_app` to

```rust
//! tests/health_check.rs
// [...]

async fn spawn_app() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to address");
    let server = zero2prod::run(listener).expect("Failed to start server");
    let _server_handle = tokio::spawn(server.into_future());
}
```

Done - the background app now runs on a random port every time we launch `cargo test`!
There is only a small issue... our test is failing!

```sh
running 1 test
test health_check_works ... FAILED

failures:

---- health_check_works stdout ----

thread 'health_check_works' (33223) panicked at tests/health_check.rs:15:10:
Failed to execute request.: reqwest::Error { kind: Request, url: "http://127.0.0.1:8000/health_check", source: hyper_util::client::legacy::Error(Connect, ConnectError("tcp connect error", 127.0.0.1:8000, Os { code: 111, kind: ConnectionRefused, message: "Connection refused" })) }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    health_check_works

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

Our HTTP client is still calling `127.0.0.1:8000` and we really don't know what to put there now: the application port is determined at runtime, we cannot hard-code it there.

We need, somehow, to find out what port the OS has gifted our application and return it from `spawn_app`.

In axum, `TcpListener` (from tokio) already gives us this information: `listener.local_addr()` returns a `SocketAddr` which exposes the actual port we bound via `.port()`. Since our `run` function already accepts a `tokio::net::TcpListener`, we can read the port _before_ passing the listener into `run`.

What is the upside? We retrieve the port immediately after binding, before the server takes ownership of the listener, and return the application's base URL to the caller.

Let's update `run` — it stays the same, no changes needed there. The only change is in `spawn_app`:

```rust
//! tests/health_check.rs
// [...]

use tokio::net::TcpListener;

async fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to address");
    let port = listener.local_addr().unwrap().port();
    let server = zero2prod::run(listener).expect("Failed to start server");
    let _server_handle = tokio::spawn(server.into_future());

    format!("http://127.0.0.1:{}", port)
}
```

We can now leverage the application address returned by `spawn_app` in our test to point our `reqwest::Client`:

```rust
//! tests/health_check.rs
// [...]

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let address = spawn_app().await;

    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(format!("{}/health_check", &address))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length())
}
```

All is good — `cargo test` comes out green. Our setup is much more robust now!

---

## 3.6 Refocus

---

Let's take a small break to look back — we covered a fair amount of ground!

We set out to implement a `/health_check` endpoint and that gave us the opportunity to learn more about the fundamentals of our web framework, axum, as well as the basics of (integration) testing for Rust APIs.

It is now time to capitalise on what we learned to finally fulfill the first user story of our email newsletter project:

> As a blog visitor,
> I want to subscribe to the newsletter,
> So that I can receive email updates when new content is published on the blog.

We expect our blog visitors to input their email address in a form embedded on a web page. The form will trigger a `POST /subscriptions` call to our backend API that will actually process the information, store it and send back a response.

We will have to dig into:

- how to read data collected in a HTML form in axum (i.e. how do I parse the request body of a POST?);
- what libraries are available to work with a PostgreSQL database in Rust (diesel vs sqlx vs tokio-postgres);
- how to setup and manage migrations for our database;
- how to get our hands on a database connection in our API request handlers;
- how to test for side-effects (a.k.a. stored data) in our integration tests;
- how to avoid weird interactions between tests when working with a database.

Let's get started!

---

## 3.7 Working with HTML Forms

---

### 3.7.1 Refining Our Requirements

// Everything as it is

### 3.7.2 Capturing Our Requirements As Tests

Now that we understand better what needs to happen, let's encode our expectation in a couple of integration tests.

Let's add the new tests to the existing `tests/health_check.rs` file - we will re-organize our test suite folder structure afterwards.

```rust
//! tests/health_check.rs
use tokio::net::TcpListener;

/// Spawns the application and returns the address (including port) that it is listening on.
///
/// The application is spawned on a random available port to avoid conflicts with other tests or
/// applications.
///
/// # Returns
///
/// A `String` containing the full address (including port) that the application is listening on,
/// e.g., "http://127.0.0.1:XXXX"
async fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to address");
    let port = listener.local_addr().unwrap().port();
    let server = zero2prod::run(listener).expect("Failed to start server");
    let _server_handle = tokio::spawn(server.into_future());

    format!("http://127.0.0.1:{}", port)
}

#[tokio::test]
async fn health_check_works() {
    [...]
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let app_address = spawn_app().await;
    let client = reqwest::Client::new();
    // Act
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(format!("{}/subscriptions", &app_address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");
    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_returns_a_422_when_data_is_missing() {
    // Arrange
    let app_address = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];
    for (invalid_body, error_message) in test_cases {
        // Act
        let response = client
            .post(format!("{}/subscriptions", &app_address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");
        // Assert
        assert_eq!(
            422,
            response.status().as_u16(),
            // Additional customized error message on test failure
            "The API did not fail with 422 Unprocessable Entity when the payload was {}.",
            error_message
        );
    }
}
```

`subscribe_returns_a_422_when_data_is_missing` is an example of a table-driven test, also known as a parametrised test.

It is particularly helpful when dealing with bad inputs — instead of duplicating test logic several times we can simply run the same assertion against a collection of known invalid bodies that we expect to fail in the same way.

With parametrised tests it is important to have good error messages on failures: `assertion failed on line XYZ` is not great if you cannot tell which specific input is broken! On the flip side, that parametrised test is covering a lot of ground so it makes sense to invest a bit more time in generating a nice failure message.

Test frameworks in other languages sometimes have native support for this testing style (e.g. parametrised tests in `pytest` or `InlineData` in xUnit for C#); in the Rust ecosystem, you can get similar functionality via a third-party crate, `rstest`.

Let's run our test suite now:

```sh
---- health_check::subscribe_returns_a_200_for_valid_form_data stdout ----
thread 'health_check::subscribe_returns_a_200_for_valid_form_data'
panicked at 'assertion failed: `(left == right)`
left: `200`,
right: `404`:
---- health_check::subscribe_returns_a_422_when_data_is_missing stdout ----
thread 'health_check::subscribe_returns_a_422_when_data_is_missing'
panicked at 'assertion failed: `(left == right)`
left: `422`,
right: `404`:
The API did not fail with 422 Unprocessable Entity when the payload was missing the email.'
```

As expected, all our new tests are failing.

You can immediately spot a limitation of "roll-your-own" parametrised tests: as soon as one test case fails, the execution stops and we do not know the outcome for the following test cases.

Let's get started on the implementation.

### 3.7.3 Parsing Form Data From A `POST` Request

All tests are failing because the application returns a `404 NOT FOUND` for `POST` requests hitting `/subscriptions`. Legitimate behaviour: we do not have a handler registered for that path.

Let's fix it by adding a matching route to our `Router` in `src/lib.rs`:

```rust
//! src/lib.rs
use axum::{
    Router,
    http::StatusCode,
    routing::{get, post},
    serve::Serve,
};
use tokio::net::TcpListener;

// We were returning `impl IntoResponse` before.
// We are not spelling out the type explicitly given that we have become more familiar with `axum`.
// There is no performance difference! Just a stylistic choice :)
async fn health_check() -> StatusCode {
    StatusCode::OK
}

// Let's start simple: we always return a 200 OK
async fn subscribe() -> StatusCode {
    StatusCode::OK
}

pub fn run(listener: TcpListener) -> Result<Serve<TcpListener, Router, Router>, std::io::Error> {
    let app = Router::new()
        .route("/health_check", get(health_check))
        // A new entry in our router table for POST /subscriptions requests
        .route("/subscriptions", post(subscribe));
    let server = axum::serve(listener, app);
    Ok(server)
}

```

Running our test suite again:

```sh
running 3 tests
test health_check_works ... ok
test subscribe_returns_a_200_for_valid_form_data ... ok
test subscribe_returns_a_422_when_data_is_missing ... FAILED

failures:

---- subscribe_returns_a_422_when_data_is_missing stdout ----

thread 'subscribe_returns_a_422_when_data_is_missing' (23270) panicked at tests/health_check.rs:82:9:
assertion `left == right` failed: The API did not fail with 422 Unprocessable Entity when the payload was missing the email.
  left: 422,
 right: 200
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    subscribe_returns_a_400_when_data_is_missing

test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
```

`subscribe_returns_a_200_for_valid_form_data` now passes: well, our handler accepts **all** incoming data as valid, no surprises there.

`subscribe_returns_a_400_when_data_is_missing`, instead, is still red.

Time to do some real parsing on that request body. What does axum offer us?

#### 3.7.3.1 Extractors

Quite prominent in axum's documentation is the [**Extractors**](https://docs.rs/axum/latest/axum/#extractors) section.

Extractors are used, as the name implies, to tell the framework to extract certain pieces of information from an incoming request.

Extractors are how you pick apart the incoming request to get the parts your handler needs.

axum provides several extractors out of the box to cater for the most common use cases:

- `Path` to get dynamic path segments from a request's path;
- `Query` for query parameters;
- `Json` to parse a JSON-encoded request body;
- `HeaderMap` to get access to the request's headers;
- `String` to get the raw request body as a string; It consumes the entire body and ensures it is valid UTF-8.
- `Bytes` gives you the raw request body
- `Request` gives you the whole request for maximum control
- `Extension` extracts data from "request extensions". This is commonly used to share state across handlers.

Besides these
[`Form`](https://docs.rs/axum/latest/axum/struct.Form.html#as-extractor) can also be used as extractor to extract url-encoded form data from the request body. This is only available if you enable the `form` feature of axum. Which can be done by modifying the axum line in our `Cargo.toml`:

```toml
[dependencies]
axum = { version = "0.8.8", features = ["form"] }
```

That's music to my ears.

How do we use it?

In axum, an extractor is simply a type that implements the `FromRequest` or `FromRequestParts` trait. You use it by declaring it as a parameter of your handler function — axum will deserialise the incoming request into it automatically. Argument position does not matter; axum will figure out what each argument needs.

Example:

```rust
use axum::Form;

#[derive(serde::Deserialize)]
struct FormData {
    username: String,
}

/// Extract form data using serde.
/// This handler gets called only if the content type is *x-www-form-urlencoded*
/// and the content of the request could be deserialized to a `FormData` struct.
async fn index(Form(form): Form<FormData>) -> String {
    format!("Welcome {}!", form.username)
}
```

So, basically… you just declare it as an argument of your handler and axum, when a request comes in, will do the heavy-lifting for you. Let's ride along for now and we will circle back later to understand what is happening under the hood.

Our `subscribe` handler currently looks like this:

```rust
//! src/lib.rs
// Let's start simple: we always return a 200 OK
async fn subscribe() -> StatusCode {
    StatusCode::OK
}
```

Using the example as a blueprint, we probably want something along these lines:

```rust
//! src/lib.rs
// [...]
#[derive(serde::Deserialize)]
struct FormData {
    email: String,
    name: String,
}

async fn subscribe(_form: Form<FormData>) -> StatusCode {
    StatusCode::OK
}
```

`cargo check` is not happy:

```sh
error[E0433]: failed to resolve: use of unresolved module or unlinked crate `serde`
  --> src/lib.rs:14:10
   |
14 | #[derive(serde::Deserialize)]
   |          ^^^^^ use of unresolved module or unlinked crate `serde`

For more information about this error, try `rustc --explain E0433`.
```

Fair enough: we need to add `serde` to our dependencies. Let's add a new line to our `Cargo.toml`:

```toml
[dependencies]
# We need the optional `derive` feature to use `serde`'s procedural macros:
# `#[derive(Serialize)]` and `#[derive(Deserialize)]`.
# The feature is not enabled by default to avoid pulling in
# unnecessary dependencies for projects that do not need it.
serde = { version = "1.0.228", features = ["derive"] }
```

`cargo check` should succeed now. What about `cargo test`?

```sh
running 3 tests
test health_check_works ... ok
test subscribe_returns_a_200_for_valid_form_data ... ok
test subscribe_returns_a_400_when_data_is_missing ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
```

They are all green!

But **why?**

#### 3.7.3.2 `Form` and `FromRequest`

Let's go straight to the source: what does `Form` look like?

The _`definition`_ seems fairly innocent:

```rust
#[derive(Debug, Clone, Copy, Default)]
#[must_use]
pub struct Form<T>(pub T);
```

It is nothing more than a wrapper: it is generic over a type `T` which is then used to populate `Form`'s only field.
Not much to see here.
Where does the extraction magic take place?

An extractor is a type that implements the `FromRequest` or `FromRequestParts` trait. Let's check if `Form` implements one of those traits:

```rust
/// Types that can be created from requests.
///
/// Extractors that implement `FromRequest` can consume the request body and can thus only be run
/// once for handlers.
///
/// If your extractor doesn't need to consume the request body then you should implement
/// [`FromRequestParts`] and not [`FromRequest`].
pub trait FromRequest<S, M = private::ViaRequest>: Sized {
    /// If the extractor fails it'll use this "rejection" type. A rejection is
    /// a kind of error that can be converted into a response.
    type Rejection: IntoResponse;

    /// Perform the extraction.
    fn from_request(
        req: Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send;
}
```

`from_request` takes as input the incoming request (i.e. `Request`) and the application state (if any) and returns a future that will resolve to either an instance of the extractor type or a rejection. It then returns `Self`, if the extraction was successful, or a `Rejection` if it failed.

All arguments in the signature of a route handler must implement `FromRequest` or `FromRequestParts` for axum to be able to call the handler when a request comes in. If any of the arguments does not implement one of those traits, you will get a compile-time error.
Axum will invoke appropriate `from_request` or `from_request_parts` implementations to extract the arguments from the incoming request before calling the handler.
If the extraction process fails for any argument, axum will convert the corresponding `Rejection` into an HTTP response and return it to the client without calling the handler.

Let's look at `Form`'s `FromRequest` implementation: what does it do?
Once again, I slightly reshaped the _`actual code`_ to highlight the key elements and ignore the nitty gritty implementation details.

```rust
impl<T, S> FromRequest<S> for Form<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = FormRejection;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {

        // Step 1: Read the body and validate Content-Type header
        // (omitted: content-type check, body size limits, streaming)
        let RawForm(bytes) = req.extract().await?;

        // Step 2: Percent-decode the bytes and deserialize into T via serde
        // (omitted: path-aware error tracking, GET vs POST error distinction)
        let value = serde_urlencoded::from_bytes::<T>(&bytes)
            .map_err(|e| FormRejection::from(e))?;

        Ok(Form(value))
    }
}
```

All the heavy-lifting seems to be split across two delegates.

First, `req.extract::<RawForm>()` does a lot: it validates that the `Content-Type` header is set to `application/x-www-form-urlencoded`, deals with the fact that the request body arrives a chunk at a time as a stream of bytes, collects it all into a contiguous buffer, etc.

The key passage, after all those things have been taken care of, is:

```rust
serde_urlencoded::from_bytes::<T>(&bytes)
    .map_err(|e| FormRejection::from(e))
```

`serde_urlencoded` provides (de)serialisation support for the `application/x-www-form-urlencoded` data format.

`from_bytes` takes as input a contiguous slice of bytes and it deserialises an instance of type `T` from it according to the rules of the URL-encoded format: the keys and values are encoded in key-value tuples separated by `&`, with a `=` between the key and the value; non-alphanumeric characters in both keys and values are percent encoded.

How does it know how to do it for a generic type `T`?

It is because `T` implements the `DeserializeOwned` trait from `serde`:

```rust
impl<T, S> FromRequest<S> for Form<T>
where
    T: DeserializeOwned,
    // [...]
```

To understand what is actually happening under the hood we need to take a closer look at `serde` itself.

> The next section on serde touches on a couple of advanced Rust topics.
> It’s fine if not everything falls into place the first time you read it!
> Come back to it once you have played with Rust and serde a bit more to deep-dive on the toughest bits of it.

#### 3.7.3.3 Serialization In Rust: `serde`

// Everything as it is

#### 3.7.3.4 Putting Everything Together

Given everything we learned so far, let's take a second look at our `subscribe` handler:

```rust
#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

// Let's start simple: we always return a 200 OK
async fn subscribe(_form: Form<FormData>) -> StatusCode {
    StatusCode::OK
}
```

We now have a good picture of what is happening:

- before calling `subscribe`, axum invokes the `from_request` method for all of `subscribe`'s input arguments: in our case, `Form::from_request`;
- `Form::from_request` tries to deserialise the body into `FormData` according to the rules of URL-encoding, leveraging `serde_urlencoded` and the `Deserialize` implementation of `FormData`, automatically generated for us by `#[derive(serde::Deserialize)]`;
- if `Form::from_request` fails due to empty data a `422 Unprocessable Entity` is returned to the caller. If it succeeds, `subscribe` is invoked and we return a `200 OK`.

Take a moment to be amazed: it looks so deceptively simple, yet there is so much going on in there — we are leaning heavily on Rust's strengths as well as some of the most polished crates in its ecosystem.

---

## 3.8 Storing Data: Databases

---

// Intro as it is

### 3.8.1 Choosing A Database

// Everything as it is

### 3.8.2 Choosing A Database Crate

As of March 2026, there are four top-of-mind options when it comes to interacting with PostgreSQL in a Rust project:

- `tokio-postgres`
- `sqlx`
- `diesel`
- `SeaORM`

For this book we will use `diesel` with `diesel-async`.

With `diesel`, you build queries using a type-safe Rust DSL generated from your database schema. This may sound more involved than writing raw SQL, but it comes with a meaningful upside: your queries are expressed as ordinary Rust code and inherit all of the guarantees the compiler can offer.

SQL is fraught with pitfalls, though. It is fairly easy to make mistakes when writing queries. We might, for example,

- have a typo in the name of a column or a table mentioned in our query;
- try to perform operations that are rejected by the database engine (e.g. summing a string and a number
  or joining two tables on the wrong column);
- expect to have a certain field in the returned data that is actually not there.

The key question is: **when** do we realise we made a mistake?

In most programming languages, it will be at **runtime**: when we try to execute our query the database will reject it and we will get an error or an exception. `diesel` speeds up the feedback cycle by catching most of these mistakes **at compile time**. It works by running `diesel print-schema` (or `diesel migration run`) to generate a `schema.rs` file that encodes your database table structure as Rust types. The DSL methods — `filter`, `select`, `inner_join`, and friends — are all generic over those generated types, so referring to a non-existent column, mismatching types across a join, or expecting a field that is not in the schema
all become compiler errors rather than runtime surprises.

Enough with the introduction though, it'll be easier to understand how `diesel` works by looking at it in action.

### 3.8.3 Integration Testing With Side-effects

// Everything as it is

### 3.8.4 Database Setup

// Intro as it is

#### 3.8.4.1 Docker

// Everything as it is

#### 3.8.4.2 User Management

// Everything as it is

#### 3.8.4.3 Database Migration

To store our subscriber details we need to create our first table.
To add a new table to our database we need to change its _`schema`_ - this is commonly referred to as a _database migration_.

##### 3.8.4.3.1 `diesel_cli`

`diesel` provides a command-line tool, `diesel_cli`, that helps us manage our database schema. It can be installed with:

```sh
cargo install diesel_cli --locked --no-default-features --features postgres
```

This may sometime throw an error about `libpq` not being found. In that case installing `libpq-dev` with your system's package manager should do the trick.

For example, on Debian-based Linux distributions you can run:

```sh
sudo apt-get install libpq-dev
```

##### 3.8.4.3.2 Database Creation

The first command we will usually want to run is `diesel setup`.

According to the `diesel` cli help, the `setup` command does the following:

```sh
diesel setup -h
Creates the migrations directory, creates the database specified in your DATABASE_URL, and runs existing migrations.
```

This will create the database specified in our `DATABASE_URL` environment variable and run any pending migrations (we don't have any yet, so it won't do much at this point). It also creates a `migrations` directory in our project where we will keep all of our migration files. By default, it also creates on initial migration which establishes starting point for database schema. You shouldn't delete it. It just keeps two utility function in our database.

You can also add the environment variable to your `.env` file, and `diesel` will pick it up automatically. Mind you this must be a valid connection string for your database, otherwise you will get an error when running `diesel setup`:

```.env
DATABASE_URL=postgres://postgres:password@localhost/newsletter
```

Now we can add couple of lines in our `scripts/init_db.sh` file for automatically running `diesel setup`

```sh
# [...]

DATABASE_URL=postgres://${APP_USER}:${APP_PASSWORD}@localhost/${APP_DB}
export DATABASE_URL
diesel setup
```

This will also create a `diesel.toml` file in the root of our project with the following content:

```toml
# For documentation on how to configure this file,
# see https://diesel.rs/guides/configuring-diesel-cli

[print_schema]
file = "src/schema.rs"
custom_type_derives = ["diesel::query_builder::QueryId", "Clone"]

[migrations_directory]
dir = "migrations"
```

This can be used to configure the behavior of `diesel_cli` - for example, we can change the location of the generated `schema.rs` file or the directory where our migration files are stored.

Scripts do not come bundled with a manifest to declare their dependencies: it's unfortunately very common to launch a script without having installed all the prerequisites. This will usually result in the script crashing mid-execution, sometimes leaving stuff in our system in a half-broken state. We can do better in our initialization script: let's check if `diesel-cli` is installed at the very beginning.

```sh
set -x
set -eo pipefail

if ! [ -x "$(command -v diesel)" ]; then
    echo >&2 "Error: diesel_cli is not installed."
    echo >&2 "Use:"
    echo >&2 "    cargo install diesel_cli --locked --no-default-features --features postgres"
    echo >&2 "to install it."
    exit 1
fi

# The rest of the script goes here
```

##### 3.8.4.3.3 Adding a migration

Let's create our first migration now with

```sh
# Assuming you have set DATABASE_URL with default parameters of the script in .env or in environment variables
diesel migration generate create_subscriptions_table
```

Migrations allow us to evolve the database schema over time. Each migration consists of an **up.sql** file to apply the changes and a **down.sql** file to revert them. Applying and immediately reverting a migration should leave your database schema unchanged.

A new directory will be created in `migrations` with a timestamp and the name of the migration, e.g. `{timestamp}-0000_create_subscriptions_table`. Inside it, you will find the two files mentioned above.
Format of the timestamp is `{year}-{month}-{day}-{time:HHMMSS}`. The timestamp is used to determine the order of the migrations: they will be applied in chronological order.
Let's edit the `up.sql` file to create a `subscriptions` table with the following schema:

```sql
-- migrations/{timestamp}_create_subscriptions_table/up.sql
-- This is the "up" migration for creating the subscriptions table
CREATE TABLE subscriptions (
    id uuid NOT NULL,
    PRIMARY KEY (id),
    email TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    subscribed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

Now for `down.sql`:

```sql
-- migrations/{timestamp}_create_subscriptions_table/down.sql
-- This is the "down" migration for dropping the subscriptions table
DROP TABLE IF EXISTS subscriptions;
```

There is an [endless debate](https://z2p.io/fxh) when it comes to [primary keys](https://z2p.io/fxg): some people prefer to use columns with a business meaning (e.g. email, a _natural key_), others feel safer with a synthetic key without any business meaning (e.g. id, a randomly generated UUID, a _surrogate key_).

I generally default to a synthetic identifier unless I have a very compelling reason not to - feel free to disagree with me here.

A couple of other things to make a note of:

- we are keeping track of when a subscription is created with `subscribed_at` (`TIMESTAMPTZ` is a timezone aware date and time type);
- we are enforcing email uniqueness at the database-level with a `UNIQUE` constraint;
- we are enforcing that all fields should be populated with a `NOT NULL` constraint on each column;
- we are using `TEXT` for email and name because we do not have any restriction on their maximum lengths.

Database constraints are useful as a last line of defence from application bugs but they come at a cost - the database has to ensure all checks pass before writing new data into the table. Therefore constraints impact our write-throughput, i.e. the number of rows we can `INSERT`/`UPDATE` per unit of time in a table.

`UNIQUE`, in particular, introduces an additional B-tree index on our `email` column: the index has to be updated on every `INSERT`/`UPDATE`/`DELETE` query and it takes space on disk.

In our specific case, I would not be too worried: our mailing list would have to be **incredibly popular** for us to encounter issues with our write throughput. Definitely a good problem to have, if it comes to that.

##### 3.8.4.3.4 Running Migrations

We can run migrations against our database with

```sh
diesel migration run
```

This will look at the `migrations` directory and apply any pending migrations to the database specified in our `DATABASE_URL` environment variable. It will also create a `__diesel_schema_migrations` table in our database to keep track of which migrations have been applied.
It will also create a `schema.rs` file in `src` with the following content:

```rust
// @generated automatically by Diesel CLI.

diesel::table! {
    subscriptions (id) {
        id -> Uuid,
        email -> Text,
        name -> Text,
        subscribed_at -> Timestamptz,
    }
}
```

This file is generated by `diesel` and should not be edited manually. It encodes our database schema as Rust types and is used by the `diesel` DSL to provide type safety for our queries.
This needs to be added to our `src/lib.rs` file to be available in our codebase:

```rust
// src/lib.rs
pub mod schema;

[...]
```

It's a good idea to make sure that **down.sql** is correct. You can quickly confirm that your **down.sql** rolls back your migration correctly by **redoing** the migration:

```sh
diesel migration redo
```

It has the same behavior of `diesel setup` it will look at the `DATABASE_URL` environment variable to understand what database needds to be migrated.

It will be last addition to our `scripts/init_db.sh` script, which should now look like this:

```sh
#!/usr/bin/env bash
set -x
set -eo pipefail

if ! [ -x "$(command -v diesel)" ]; then
    echo >&2 "Error: diesel_cli is not installed."
    echo >&2 "Use:"
    echo >&2 "    cargo install diesel_cli --locked --no-default-features --features postgres"
    echo >&2 "to install it."
    exit 1
fi

# Check if a custom parameter has been set, otherwise use default values
DB_PORT="${POSTGRES_PORT:=5432}"
SUPERUSER="${SUPERUSER:=postgres}"
SUPERUSER_PWD="${SUPERUSER_PWD:=password}"

APP_USER="${APP_USER:=app}"
APP_USER_PWD="${APP_USER_PWD:=secret}"
APP_DB_NAME="${APP_DB_NAME:=newsletter}"

# Launch postgres using Docker
CONTAINER_NAME="postgres_zero2prod_axum_diesel"
docker run \
    -e POSTGRES_USER="$SUPERUSER" \
    -e POSTGRES_PASSWORD="$SUPERUSER_PWD" \
    --health-cmd="pg_isready -U ${SUPERUSER} || exit 1" \
    --health-interval=1s \
    --health-timeout=5s \
    --health-retries=5 \
    -p "$DB_PORT:5432" \
    -d \
    --name "${CONTAINER_NAME}" \
    postgres:14-alpine -N 1000
    # ^ Increased maximum number of connections for testing purposes

# Wait for Postgres to be ready to accept connections
until [ \
    "$(docker inspect -f "{{.State.Health.Status}}" ${CONTAINER_NAME})" == \
    "healthy" \
]; do
    >&2 echo "Waiting for Postgres to be healthy...- sleeping for 1 second"
    sleep 1
done

>&2 echo "Postgres is healthy and ready to accept connections on PORT ${DB_PORT}!"

# Create the application user
CREATE_QUERY="CREATE USER ${APP_USER} WITH PASSWORD '${APP_USER_PWD}';"
docker exec -it "${CONTAINER_NAME}" psql -U "${SUPERUSER}" -c "${CREATE_QUERY}"

# Grant create db privileges to the app user
GRANT_QUERY="ALTER USER ${APP_USER} CREATEDB;"
docker exec -it "${CONTAINER_NAME}" psql -U "${SUPERUSER}" -c "${GRANT_QUERY}"

DATABASE_URL="postgres://${APP_USER}:${APP_USER_PWD}@localhost:${DB_PORT}/${APP_DB_NAME}"
export DATABASE_URL

diesel setup
diesel migration run

>&2 echo "Postgres has been migrated, 'ready to go!'"
```

We have put the `docker run` invocation behind the `SKIP_DOCKER` flag to make it easy to run migrations against an existing Postgres instance without having to tear it down manually and re-create it with `scripts/init_db.sh`. It will also be useful in CI, if Postgres is not spun up by our script.

We can now initialise a running database with

```sh
SKIP_DOCKER=true ./scripts/init_db.sh
```

You should be able to spot, in the output, something like

```sh
> diesel migration run
Running migration 2026-04-12-124918-0000_create_subscriptions_table
```

If you check your database using [your favourite graphic interface](https://z2p.io/f6f) for Postgres you will now see a `subscriptions` table alongside a brand new `__diesel_schema_migrations` table: this is where `diesel` keeps track of what migrations have been run against your database - it should contain a single row now for our `create_subscriptions_table` migration.

### 3.8.5 Writing Our First Query

We have migrated database up and running. How do we talk to it?

#### 3.8.5.1 `diesel` and `diesel-async`

We have installed `diesel_cli` but we have actually not yet added `diesel` as a dependency to our application. Before appending it to our `Cargo.toml` file, let's take a moment to understand what `diesel` and `diesel-async` are.

`diesel` is the core library that provides the DSL for building queries and the code generation for our schema. It is synchronous and blocking by default, which means that it will block the thread while waiting for a response from the database. This is not ideal for a web application where we want to be able to handle multiple requests concurrently without blocking the entire server.

`diesel-async` is an extension of `diesel` that provides asynchronous support. It allows us to run our queries without blocking the thread, which is essential for building a responsive web application. It has drop in replacement for diesel functionality that actually interacts with database. Notably it provides drop in replacement for the `RunQueryDsl` and `Connection` traits. It also has inbuilt integration with connection pooling libraries like `bb8` and `deadpool`.

Now let's add both `diesel` and `diesel-async` to our `Cargo.toml` file:

```toml
[dependencies]
diesel = { version = "2.3.7", features = ["chrono", "uuid"] }
diesel-async = { version = "0.8.0", features = [
  "deadpool",
  "migrations",
  "postgres"
] }
```

Or we can use cargo add:

```sh
cargo add diesel@2.3.7 --features chrono,uuid
cargo add diesel-async@0.8.0 --features deadpool,migrations,postgres
```

There are a few features flag. Let's go through them one by one:

- `chrono`: this feature enables support for the `chrono` crate, which provides date and time types. We need this to work with our `subscribed_at` column, which is of type `TIMESTAMPTZ`.
- `uuid`: this feature enables support for the `uuid` crate, which provides a type for working with UUIDs. We need this to work with our `id` column, which is of type `uuid`.
- `postgres`: this feature enables support for PostgreSQL in `diesel-async`. We need this to be able to use `diesel-async` with our PostgreSQL database.
- `migrations`: Enables the [AsyncMigrationHarness](https://docs.rs/diesel-async/0.8.0/diesel_async/struct.AsyncMigrationHarness.html) to execute migrations via [diesel_migrations](https://docs.rs/diesel_migrations/2.3.1/diesel_migrations/)
- `deadpool`: this feature enables support for the [`deadpool`](https://docs.rs/deadpool/0.13.0/x86_64-unknown-linux-gnu/deadpool/index.html) connection pooling library. We will use `deadpool` to manage our database connections in an efficient way.

This should be enough for now.

#### 3.8.5.2 Configuration Management

The simplest entrypoint to connect to a Postgres database with `diesel-async` is [`AsyncPgConnection`](https://docs.rs/diesel-async/latest/diesel_async/struct.AsyncPgConnection.html).

`AsyncPgConnection` can be created via `AsyncPgConnection::establish`, which takes a connection string and returns a `Result<AsyncPgConnection, ConnectionError>`. This doesn't natively support TLS connections. We will cover later on how to setup TLS connections.

Where do we get a connection string?

We could hard-code one in our application and then use it for our tests as well. Or we could choose to introduce immediately some basic mechanism of configuration management.

It is simpler than it sounds and it will save us the cost of tracking down a bunch of hard-coded values across the whole application.

The [`config`](https://docs.rs/config/latest/config/) crate is Rust's swiss-army knife when it comes to configuration: it supports multiple file formats and it lets you combine different sources hierarchically (e.g. environment variables, configuration files, etc.) to easily customise the behaviour of your application for each deployment environment.

We do not need anything fancy for the time being: a single configuration file will do.

##### 3.8.5.2.1 Making Space

// Everything as it is

##### 3.8.5.2.2 Reading a Configuration File

To manage configuration with `config` we must represent our application settings as a Rust type that implements `serde`'s `Deserialize` trait.
Let's create a new `Settings` struct:

```rust
//! src/configuration.rs
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Settings {}
```

We have two groups of configuration values at the moment:

- the application port, where `axum` is listening for incoming requests (currently hard-coded to `8000` in `main.rs`);
- the database connection parameters.

Let's add a field for each of them to `Settings`:

```rust
//! src/configuration.rs
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application_port: u16,
}

#[derive(Deserialize, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}
```

We need `#[derive(Deserialize)]` on top of `DatabaseSettings` otherwise the compiler will complain with

```sh
error[E0277]: the trait bound
    `configuration::DatabaseSettings: configuration::_::_serde::Deserialize<'_>`
    is not satisfied
 --> src/configuration.rs:3:5
  |
3 |     pub database: DatabaseSettings,
  |     ^^^ the trait `configuration::_::_serde::Deserialize<'_>`
  |         is not implemented for `configuration::DatabaseSettings`
  |
  = note: required by `configuration::_::_serde::de::SeqAccess::next_element`
```

It makes sense: all fields in a type have to be deserialisable in order for the type as a whole to be deserialisable.

We have our configuration type, what now?

First of all, let's add `config` to our dependencies with

```toml
#! Cargo.toml
# [...]
[dependencies]
config = "0.15.22"
# [...]
```

or

```sh
cargo add config@0.15.22
```

We want to read our application settings from a configuration file named `configuration.yaml`:

```rust
//! src/configuration.rs
// [...]
pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    // Initialise our configuration reader
    let settings = config::Config::builder()
        // Add configuration values from a file named `configuration.yaml`.
        .add_source(
            config::File::new("configuration.yaml", config::FileFormat::Yaml)
        )
        .build()?;

    // Try to convert the configuration values it read into
    // our Settings type
    settings.try_deserialize::<Settings>()
}
```

Let's modify our `main` function to read configuration as its first step:

```rust
//! src/main.rs
use tokio::net::TcpListener;
use zero2prod::{configuration::get_configuration, run};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Panic if we can't read configuration
    let configuration = get_configuration().expect("Failed to read configuration.");

    // we have removed the hard-coded `8000` - it's now coming from our settings!
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address).await?;
    run(listener)?.await
}
```

If you try to launch the application with `cargo run` it should crash:

```sh
     Running `target/debug/zero2prod`

thread 'main' (17014) panicked at src/main.rs:7:45:
Failed to read configuration.: configuration file "configuration.yaml" not found
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

Let's fix it by adding a configuration file.

We can use any file format for it, as long as `config` knows how to deal with it: we will go for YAML.

```yaml
# configuration.yaml
application_port: 8000
database:
  host: "127.0.0.1"
  port: 5432
  username: "postgres"
  password: "password"
  database_name: "newsletter"
```

`cargo run` should now execute smoothly.

#### 3.8.5.3 Connecting to Postgres

There is two steps to connect to Postgres without TLS with `diesel-async` and `deadpool`.
First create a connection manager `AsyncDieselConnectionManager` which takes in database connection string as a single string. And then create a connection pool `Pool` with the connection manager as a parameter.

But `DatabaseSettings` provides us with a granular access to all the connection parameters. Let's add a convenient `connection_string` method to do it:

```rust
//! src/configuration.rs
// [...]
impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database_name
        )
    }
}
```

We are finally ready to connect!
Let's tweak our happy case test:
In case of test let's connect to database without `deadpool` connection pool for simplicity. We will add connection pooling later.
Direct connection is established with `AsyncPgConnection::establish` method, which takes in database connection string and returns a `Result<AsyncPgConnection, ConnectionError>`.

```rust
//! tests/health_check.rs
use diesel_async::{AsyncConnection, AsyncPgConnection};
use zero2prod::configuration::get_configuration;
// [...]

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let app_address = spawn_app().await;
    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_string = configuration.database.connection_string();
    let connection = AsyncPgConnection::establish(&connection_string)
        .await
        .expect("Failed to connect to Postgres.");
    let client = reqwest::Client::new();

    // Act
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(format!("{}/subscriptions", &app_address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(200, response.status().as_u16());
}
```

And... `cargo test` works!
We just confirmed that we can successfully connect to Postgres from our tests!
A small step for the world, a huge leap forward for us.

#### 3.8.5.4 Our Test Assertion

Now that we are connected, we can finally write the test assertions we have been dreaming about for the past 10 pages.
We will use `diesel` DSL to build a query that checks if the subscription we just added is actually in the database.
And we will import `diesel_async`'s `RunQueryDsl` trait to be able to execute our query asynchronously.
We have used `select` to specify the columns we want to retrieve from the database, `first` to get the first result of the query and `expect` to panic if there is an error executing the query or if there are no results.
We need to provide `<(String, String)>` as a type annotation to `first` because it needs to know what type to deserialize the result into and it cannot infer it from the context.

```rust
//! tests/health_check.rs
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use zero2prod::schema::subscriptions;
// [...]

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // [...]

    // The connection has to be marked as mutable, since `RunQueryDsl` methods take `&mut connection`
    let mut connection = AsyncPgConnection::establish(&connection_string)
        .await
        .expect("Failed to connect to Postgres.");

    // Assert
    assert_eq!(200, response.status().as_u16());

    // Assert
    assert_eq!(200, response.status().as_u16());

    // Get saved subscriber for database
    let (email, name) = subscriptions::table
        .select((subscriptions::email, subscriptions::name))
        .first::<(String, String)>(&mut connection)
        .await
        .expect("Failed to get saved subscription.");

    assert_eq!(email, "ursula_le_guin@gmail.com");
    assert_eq!(name, "le guin");
```

We have given `<(String, String)>` to make sure that return type of the query is a tuple of two strings, which is what we expect to get from the database. The first string is the email and the second string is the name.

Keep in mind if `schema.rs` file is not up to date with the database schema, you might get an error. `Diesel` relies on `schema.rs` to provide type safety for our queries, so if there is a mismatch between the actual database schema and the generated `schema.rs` file, you might get an error when trying to execute a query. If you encounter such an error, make sure to run `diesel migration run` to update your database schema and regenerate the `schema.rs` file.

Let's try to run `cargo test` again:

```sh
     Running tests/health_check.rs (target/debug/deps/health_check-7e280d9396808847)

running 3 tests
test subscribe_returns_a_400_when_data_is_missing ... ok
test health_check_works ... ok
test subscribe_returns_a_200_for_valid_form_data ... FAILED

failures:

---- subscribe_returns_a_200_for_valid_form_data stdout ----

thread 'subscribe_returns_a_200_for_valid_form_data' (72748) panicked at tests/health_check.rs:76:10:
Failed to get saved subscription.: NotFound
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    subscribe_returns_a_200_for_valid_form_data
```

It failed, which is exactly what we wanted!
We can now focus on patching the application to turn it green.

#### 3.8.5.5 Updating Our CI Pipeline

If you check on it, you will notice that your CI pipeline is now failing to perform most of the checks we introduced at the beginning of our journey.

Our tests now rely on a running Postgres database to be executed properly.

We do not want to venture further with a broken CI.

You can find an updated version of the GitHub Actions setup [on GitHub](https://github.com/shubhenduanupamdutta/Zero2Prod_Axum_Diesel/tree/root-chapter03-part0/.github/workflows).

Please keep in mind you need to add following secrets to your GitHub repository for the CI pipeline to work:

- `APP_PASSWORD`: **secret** or whatever you have used in your `scripts/init_db.sh` script for the application user password
- `APP_USER`: **app** or whatever you have used in your `scripts/init_db.sh` script for the application user username
- `PG_USER`: **postgres** or whatever you have used in your `scripts/init_db.sh` script for the Postgres superuser username
- `PG_PASSWORD`: **password** or whatever you have used in your `scripts/init_db.sh` script for the Postgres superuser password. This would also be same in `configuration.yaml` file for the database password.
