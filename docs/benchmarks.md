# Benchmarks

Performance is the cornerstone of Dapil. These benchmarks compare Dapil against industry standards and other high-performance frameworks.

## Hardware & Environment
- **CPU**: x86_64 Linux
- **Concurrency**: 50 concurrent connections
- **Total Requests**: 10,000
- **Build Mode**: Release (LTO enabled, 1 codegen unit)

## Throughput Comparison

| Framework | Requests/Sec | Latency (Mean) |
| :--- | :--- | :--- |
| **Dapil (Extreme)** | **29,661** | **1.6 ms** |
| Django-Bolt | 19,511 | 2.5 ms |
| Django (Gunicorn) | 5,623 | 8.8 ms |
| FastAPI (Uvicorn) | 3,563 | 14.0 ms |

### Analysis

1.  **Vs FastAPI (8.32x)**: FastAPI (and Uvicorn) are bottlenecked by Python's asynchronous overhead and GIL management when handling concurrent I/O. Dapil offloads all I/O to Rust, leaving Python only responsible for the business logic.
2.  **Vs Django-Bolt (1.52x)**: While Django-Bolt also uses a Rust core, Dapil's specialized **Single Actor** worker model provides lower overhead for simple dispatching, ensuring the GIL is held optimally.
3.  **Vs Standard Django (5.27x)**: Standard sync Django is limited by worker process/thread overhead. Dapil's hybrid model provides the efficiency of an async event loop with the simplicity of sync handlers.

## Reproducing Benchmarks

We use `ab` (Apache Benchmark) for reliable throughput measurement.

### 1. Run the Dapil server
```bash
python examples/hello_world/main.py
```

### 2. Execute Benchmark
```bash
ab -n 10000 -c 50 http://127.0.0.1:8181/
```

### 3. Comparison Scripts
You can find the scripts used for other frameworks in the `benchmarks/` directory:
- `fastapi_app.py`
- `django_app.py`
