from dapil import App, Response, HTTPException, StreamingResponse
import logging

logging.basicConfig(level=logging.INFO)
app = App()

@app.get("/")
def hello():
    return "Hello from Dapil (Vanilla)"

@app.get("/custom_response")
def custom():
    return Response(
        content="Custom Response Content",
        status_code=201,
        headers={"X-Custom-Header": "Dapil-Value"}
    )

@app.get("/error")
def error():
    raise HTTPException(status_code=400, detail="Custom Error Message")

@app.get("/json")
def json_resp():
    import json
    return Response(
        content=json.dumps({"message": "Hello JSON"}),
        media_type="application/json"
    )

@app.get("/stream")
def stream():
    import time
    def generator():
        for i in range(5):
            yield f"chunk {i}\n"
            time.sleep(0.5)
    return StreamingResponse(generator(), media_type="text/plain")

if __name__ == "__main__":
    app.set_port(8184)
    app.serve()
