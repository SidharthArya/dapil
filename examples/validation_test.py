from dapil import Dapil, Request
from pydantic import BaseModel
from typing import Optional
import json

app = Dapil()

class User(BaseModel):
    id: int
    name: str
    email: Optional[str] = None

@app.post("/users")
async def create_user(user: User):
    print(f"Creating user: {user}")
    return {"message": "User created", "user": user.dict()}

@app.get("/items/{item_id}")
async def get_item(item_id: int, q: Optional[str] = None):
    print(f"Getting item: {item_id}, q={q}")
    return {"item_id": item_id, "q": q}

@app.get("/debug")
async def debug_request(request: Request):
    body = await request.body()
    return {
        "method": request.method,
        "url": str(request.url),
        "headers": dict(request.headers),
        "query_params": request.query_params,
        "body_len": len(body)
    }

if __name__ == "__main__":
    app.host("127.0.0.1").port(8000).workers(1).serve()
