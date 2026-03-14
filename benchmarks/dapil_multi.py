
import dapil

app = dapil.App()
app.set_port(8181)

@app.get("/")
def hello():
    return "Hello from Dapil!"

if __name__ == "__main__":
    app.serve(workers=4)
