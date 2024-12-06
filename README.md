# Dapil
[Under Active Development]

A simple http server for python leveraging axum.

Disclaimer: Currently only Hello World is implemented.
# Example Usage
```python
import dapil
import asyncio

app = dapil.App()
app.set_host("127.0.0.1")
app.set_port(8181)
app.serve()

```


# Features
- [ ] Routing
- [ ] Views
- [ ] Database Models

# Building
As currently it is not ready for real world use cases, it will need to be built manually to be used.

Run the following command to build and add dapil to your pip packages.
```bash
pip install maturin
maturin develop
```