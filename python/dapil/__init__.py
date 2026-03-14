from ._dapil import App as _App, setup_logging
from .exceptions import HTTPException
from .responses import Response, StreamingResponse
from .routing import APIRouter

class App:
    def __init__(self):
        self._app = _App()
        
    def route(self, method: str, path: str):
        def decorator(func):
            self._app.route(method, path, func)
            return func
        return decorator

    def get(self, path: str):
        return self.route("GET", path)

    def post(self, path: str):
        return self.route("POST", path)

    def put(self, path: str):
        return self.route("PUT", path)

    def delete(self, path: str):
        return self.route("DELETE", path)

    def include_router(self, router: APIRouter, prefix: str = ""):
        for method, path, handler in router.routes:
            full_path = prefix + path
            # Ensure path starts with /
            if not full_path.startswith("/"):
                full_path = "/" + full_path
            self._app.route(method, full_path, handler)

    def serve(self, workers: int = 1):
        self._app.set_workers(workers)
        self._app.serve()

    def serve_default(self):
        self._app.serve_default()
    
    def set_host(self, host: str):
        self._app.set_host(host)
        
    def set_port(self, port: int):
        self._app.set_port(port)
