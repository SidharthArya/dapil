# Features

Dapil combines the best of both worlds: Rust's performance and Python's productivity.

## 🚀 Extreme Performance

Dapil is built on the [Axum](https://github.com/tokio-rs/axum) networking stack and uses a [Native Async Coroutine Model](architecture.md) to maximize throughput. It is officially faster than `django-bolt`, reaching over 23,000 requests per second.

## 🔌 Dependency Injection (`Depends`)

Dapil features a powerful, recursive dependency injection system inspired by FastAPI.
- **Param-based**: Simply use `Depends(your_function)` as a default argument.
- **Recursive**: Dependencies can depend on other dependencies.
- **Cached**: Dependencies are only executed once per request.

## 📄 Automatic OpenAPI & Swagger

Dapil automatically generates OpenAPI 3.1.0 schemas for your application.
- **Interactive Docs**: Swagger UI is available out of the box at `/docs`.
- **Pydantic Support**: Automatic schema generation for request bodies using Pydantic models.
- **Dynamic Introspection**: Your signatures define your API documentation.

## 🌉 The Rust-Python Bridge

We use [PyO3](https://github.com/PyO3/pyo3) and [pyo3-async-runtimes](https://github.com/awestover80/pyo3-async-runtimes) to create a high-performance connection. This allows Python coroutines to run natively on Rust's Tokio runtime.

## 🛠️ Advanced Build System

Powered by `maturin`, Dapil is easy to build and develop.
- **Release mode** optimizations enable fat Link-Time Optimization (LTO).
- Single-unit codegen for maximum performance.
