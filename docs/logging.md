# Logging & Observability

Dapil provides high-performance logging out of the box. It bridges the powerful Rust `tracing` ecosystem with Python's standard `logging` module.

## Automatic Initialization

Logging is automatically initialized when you create a `dapil.App` instance. You don't need to call any setup functions manually.

```python
import dapil
import logging

# Configure Python logging normally
logging.basicConfig(level=logging.INFO)

app = dapil.App() # Rust-Python logging bridge starts here
```

## How it Works

1.  **Rust Side**: Dapil uses the `tracing` crate for internal performance and diagnostic events.
2.  **The Bridge**: We use `tracing-log` to convert tracing events into standard `log` records, and `pyo3-log` to forward those records into Python.
3.  **Python Side**: Python's `logging` module receives these events as if they were emitted by a standard Python package.

## Example Output

When you run a Dapil app, you'll see structured logs like this in your terminal:

```text
INFO:dapil:Dapil serving on http://0.0.0.0:8080
INFO:dapil:Incoming request: GET /
INFO:dapil:Response sent: 200 OK in 1.2ms
```

## Manual Setup (Optional)

If you need to initialize logging before creating the `App`, you can use the `setup_logging` function:

```python
import dapil
dapil.setup_logging()
```

This is useful if you want to capture logs from the initialization process itself.
