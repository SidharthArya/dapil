# Features

Dapil combines the best of both worlds: Rust's performance and Python's productivity.

## 🚀 Extreme Performance

Dapil is built on the [Axum](https://github.com/tokio-rs/axum) networking stack and uses a specialized [Single Actor GIL Model](architecture.md) to maximize throughput. This allows it to reach nearly 30,000 requests per second on a single worker thread.

## 🌉 The Rust-Python Bridge

We use [PyO3](https://github.com/PyO3/pyo3) to create a seamless connection between Rust and Python. This bridge is optimized for low-latency dispatching and efficient data conversion.

## 📊 Automatic Observability

Dapil bridges Rust's structured `tracing` events directly into Python's standard `logging` module. This means:
- No manual logging setup in Rust required.
- Logs appear in your Python log handlers automatically.
- Performance diagnostics are captured at every level of the stack.

## 🎨 Clean API Design

Inspired by FastAPI, Dapil uses a simple, decorator-based approach to routing.

- `@app.get("/path")`
- `@app.post("/path")`
- Automatic HTTP method handling.

## 🛠️ Advanced Build System

Powered by `maturin`, Dapil is easy to build and develop.
- **Release mode** optimizations enable fat Link-Time Optimization (LTO).
- Single-unit codegen for maximum performance.
