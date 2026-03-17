# Benchmarks

Performance is the cornerstone of Dapil. These benchmarks compare Dapil against industry standards and other high-performance frameworks.

## Hardware & Environment
- **CPU**: x86_64 Linux
- **Concurrency**: 50 concurrent connections
- **Total Requests**: 10,000
- **Build Mode**: Release (LTO enabled, 1 codegen unit)

## Throughput Comparison

| Framework | Requests/Sec | Latency (Mean) | Lead vs FastAPI | % of Native Rust |
| :--- | :--- | :--- | :--- | :--- |
| **Native Axum** | **47,840** | **1.1 ms** | **15.3x** | **100%** |
| **Dapil (Phase 2)** | **23,031** | **2.1 ms** | **7.3x** | **48%** |
| **Django-Bolt** | 22,180 | 2.2 ms | 7.1x | 46% |
| **FastAPI (Uvicorn)** | 3,129 | 15.9 ms | 1.0x (Baseline) | 6% |

### Analysis

1.  **Dapil vs Django-Bolt (Victory)**: Dapil is now officially **faster than Django-Bolt** for Hello World requests. By moving the Request object to Rust and eliminating Python-side `inspect` overhead, we've bypassed Bolt's architectural advantage.
2.  **Vs FastAPI (7.3x)**: FastAPI is limited by Python's asynchronous overhead and GIL management. Dapil's native async coroutine awaiting on Tokio threads provides a massive concurrency boost.
3.  **Vs Native Axum (48%)**: Dapil retains roughly **50% of the raw performance** of native Axum. This represents the total overhead of the Python execution layer. For a Python framework, 23k req/s is elite performance.

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
