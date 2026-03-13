# Dapil 🏎️💨

**Dapil** is an ultra-high-performance Python web framework powered by Rust and the [Axum](https://github.com/tokio-rs/axum) networking stack. It is designed to bridge the gap between Python's developer productivity and Rust's world-class performance.

By using a specialized **"Single Actor" GIL Model**, Dapil achieves throughput that redefines what is possible for Python web services.

## 🚀 Performance at a Glance

Dapil isn't just "fast"—it's record-breaking. In basic benchmarks, it outperforms common frameworks by massive margins:

| Framework | Requests/Sec | Lead vs FastAPI |
| :--- | :--- | :--- |
| **Dapil (Extreme)** | **29,661** | **8.3x faster** |
| Django-Bolt (Rust) | 19,511 | 5.5x faster |
| Django (Gunicorn) | 5,623 | 1.6x faster |
| FastAPI (Uvicorn) | 3,563 | 1.0x (Baseline) |

## ✨ Key Features

- **Blazing Speed**: Powered by Rust and Axum, reaching nearly 30,000 requests per second on a single worker.
- **Zero-Contention GIL Model**: Uses a dedicated single-actor thread for Python execution to eliminate lock contention.
- **Automatic Observability**: Seamlessly bridges Rust `tracing` logs to Python's standard `logging` module.
- **Modern DX**: Simple, decorator-based API inspired by FastAPI.

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

## 🧠 The "Single Actor" Secret

Most Python/Rust bridges suffer from GIL (Global Interpreter Lock) contention when scaling across threads. Dapil solves this by:
1.  Handling all network I/O in Rust's multi-threaded **Tokio** runtime.
2.  Funneling Python execution into a **single dedicated worker thread**.
3.  Passing tasks via high-performance lock-free channels.

This architecture ensures the Python interpreter spends zero time fighting for locks, allowing it to execute at 100% efficiency.

## 📖 Documentation

- [Architecture Overview](docs/architecture.md)
- [Benchmarking Guide](docs/benchmarks.md)
- [Logging & Observability](docs/logging.md)

## License
MIT