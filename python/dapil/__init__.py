from typing import Any
from ._dapil import App as _App, setup_logging
from .exceptions import HTTPException
from .responses import Response, StreamingResponse
from .requests import Request
from .middleware import BaseHTTPMiddleware as BaseMiddleware
from .routing import APIRouter

class App:
    def __init__(self):
        self._app = _App()
        self.middlewares = []
        
    def route(self, method: str, path: str):
        def decorator(func):
            self._app.route(method, path, func)
        return decorator
        
    def add_middleware(self, middleware_class: type, **options: Any):
        # We need to instantiate the middleware with the app
        # In Starlette, this is typically done by the app itself
        # Here we'll pass the class and options to the Rust core
        self._app.add_middleware_instance(middleware_class(self, **options))

    def get(self, path: str):
        return self.route("GET", path)

    def post(self, path: str):
        return self.route("POST", path)

    def put(self, path: str):
        return self.route("PUT", path)

    def delete(self, path: str):
        return self.route("DELETE", path)

    def serve(self):
        self._app.serve()

    def host(self, host: str):
        self._app.set_host(host)
        return self

    def port(self, port: int):
        self._app.set_port(port)
        return self

    def workers(self, workers: int):
        self._app.set_workers(workers)
        return self

Dapil = App
