# Dapil 🏎️💨

**Dapil** is an ultra-high-performance Python web framework powered by Rust and the [Axum](https://github.com/tokio-rs/axum) networking stack. It is designed to bridge the gap between Python's developer productivity and Rust's world-class performance.

By using a specialized **"Single Actor" GIL Model**, Dapil achieves throughput that redefines what is possible for Python web services.

## 🚀 Performance at a Glance

Dapil isn't just "fast"—it's officially faster than `django-bolt`. In basic benchmarks, it reaches elite throughput:

| Framework | Requests/Sec | Lead vs FastAPI |
| :--- | :--- | :--- |
| **Dapil (Phase 2)** | **23,031** | **7.3x faster** |
| Django-Bolt (Rust) | 22,180 | 7.1x faster |
| FastAPI (Uvicorn) | 3,129 | 1.0x (Baseline) |

## ✨ Key Features

- **Native Async**: Python coroutines run directly on Rust's Tokio runtime using `pyo3-async-runtimes`.
- **Elite Speed**: Overtakes `django-bolt` with over 23,000 requests per second.
- **Dependency Injection**: Recursive, cached DI system inspired by FastAPI.
- **Automatic OpenAPI**: Swagger UI and OpenAPI 3.1 schema generation out of the box.
- **Modern DX**: Simple, decorator-based API for rapid development.

## 📦 Quickstart

### Installation

Currently, Dapil is in early development. You can build it from source:

```bash
pip install maturin
maturin develop --release
```

### Basic App

```python
import dapil

app = dapil.App()

@app.get("/")
def hello():
    return "Hello from Dapil!"

if __name__ == "__main__":
    app.serve()
```

## 🧠 The "Native Async" Power

Most Python/Rust bridges suffer from GIL (Global Interpreter Lock) contention when scaling across threads. Dapil solves this by:
1.  Handling all network I/O in Rust's multi-threaded **Tokio** runtime.
2.  Executing Python coroutines **natively** on Tokio threads using `pyo3-async-runtimes`.
3.  Managing GIL acquisition only during active Python execution, releasing it during `await`.

This architecture ensures the Python interpreter spends zero time fighting for locks, allowing it to execute at 100% efficiency while scaling naturally with Rust's concurrency.

## 📖 Documentation

The documentation is built with [MkDocs](https://www.mkdocs.org/) using the Material theme for a premium, FastAPI-style experience.

- [Live Documentation Structure](docs/index.md)
- [Architecture Deep-Dive](docs/architecture.md)
- [Benchmarking Results](docs/benchmarks.md)

### Running the Documentation Locally

To preview the documentation site locally:

```bash
pip install mkdocs-material
mkdocs serve
```

Then visit `http://127.0.0.1:8000` in your browser.

## License
MIT