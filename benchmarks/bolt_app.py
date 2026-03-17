import os
import django
from django.conf import settings
from django_bolt import BoltAPI, _core, Depends

# Minimal Django settings
if not settings.configured:
    settings.configure(
        DEBUG=False,
        SECRET_KEY="secret",
        ROOT_URLCONF=__name__,
        INSTALLED_APPS=["django_bolt"],
        ALLOWED_HOSTS=["*"],
    )
    django.setup()

api = BoltAPI(enable_logging=False)

def get_token():
    return "secret-token"

def check_auth(token: str = Depends(get_token)):
    if token != "secret-token":
        return {"error": "Unauthorized"}
    return {"user_id": 42}

@api.get("/api/test")
async def test_route():
    return {"message": "Hello from prefixed router!"}

@api.get("/api/auth")
async def auth_route(user: dict = Depends(check_auth)):
    return {"status": "success", "user": user}

if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--workers", type=int, default=1)
    args = parser.parse_args()

    # Simulate runbolt command behavior for maximum performance
    rust_routes = []
    for method, path, handler_id, handler in api._routes:
        rust_routes.append((method, path, handler_id, handler))
    
    _core.register_routes(rust_routes)
    os.environ['DJANGO_BOLT_WORKERS'] = str(args.workers)
    
    print(f"Starting Django-Bolt on http://127.0.0.1:8084 with {args.workers} workers")
    _core.start_server_async(api._dispatch, "127.0.0.1", 8084, None)
