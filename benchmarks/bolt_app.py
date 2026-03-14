import os
import django
from django.conf import settings
from django_bolt import BoltAPI, _core

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

@api.get("/")
async def hello():
    return "Hello from Django-Bolt"

if __name__ == "__main__":
    # Simulate runbolt command behavior for maximum performance
    rust_routes = []
    for method, path, handler_id, handler in api._routes:
        rust_routes.append((method, path, handler_id, handler))
    
    _core.register_routes(rust_routes)
    os.environ['DJANGO_BOLT_WORKERS'] = '1'
    
    print("Starting Django-Bolt on http://127.0.0.1:8084")
    _core.start_server_async(api._dispatch, "127.0.0.1", 8084, None)
