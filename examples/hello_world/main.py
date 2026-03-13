import dapil
import logging

logging.basicConfig(level=logging.DEBUG, format='%(levelname)s:%(name)s:%(message)s')
# dapil.setup_logging() is no longer needed

app = dapil.App()
app.set_host("127.0.0.1")
app.set_port(8181)

@app.get("/")
def hello():
    return "Hello from Dapil Decorator!"

@app.post("/echo")
def echo():
    return "Echo Post!"

app.serve_default()
