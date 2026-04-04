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

Let's pase it in our `main.rs` file.
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
axum = 0.8
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
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
