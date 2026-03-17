from dapil import App, Depends, APIRouter
import argparse

def get_token():
    return "secret-token"

def check_auth(token: str = Depends(get_token)):
    if token != "secret-token":
        return {"error": "Unauthorized"}
    return {"user_id": 42}

app = App(title="Benchmark Features App")
router = APIRouter()

@router.get("/test")
def test_route():
    return {"message": "Hello from prefixed router!"}

@router.get("/auth")
def auth_route(user: dict = Depends(check_auth)):
    return {"status": "success", "user": user}

app.include_router(router, prefix="/api")

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--workers", type=int, default=1)
    args = parser.parse_args()
    
    app.workers(args.workers).port(8181).serve()
