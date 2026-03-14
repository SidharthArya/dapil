# Dapil 🏎️💨

<p align="center">
  <em>Ultra-high-performance Python web framework powered by Rust and Axum.</em>
</p>

---

**Dapil** is a next-generation Python web framework designed for extreme performance. It bridges the gap between Python's developer productivity and Rust's world-class efficiency.

By leveraging a specialized **Single Actor GIL Model**, Dapil achieves throughput that redefines what is possible for Python web services.

## Key Highlights

- **8x Faster than FastAPI**: Built on the world-class Axum/Tokio stack.
- **Zero GIL Contention**: Isolated worker thread ensures the Python interpreter runs at 100% efficiency.
- **Automatic Observability**: Seamless Rust-to-Python logging bridge.
- **Modern Simplicity**: Developer experience inspired by FastAPI.

## Usage Example

```python
import dapil

app = dapil.App()

@app.get("/")
def hello():
    return "Hello from Dapil!"

if __name__ == "__main__":
    app.serve()
```

## Performance

| Framework | Requests/Sec | Relative Performance |
| :--- | :--- | :--- |
| **Dapil (Extreme)** | **~29,660** | **8.3x vs FastAPI** |
| Django-Bolt (Rust) | ~19,510 | 5.5x |
| FastAPI | ~3,500 | 1.0x |

---

> [!IMPORTANT]
> Dapil is currently under active development. While performance is extreme, the feature set is still expanding.
