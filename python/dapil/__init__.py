import inspect
import functools
import asyncio
from typing import Any, Dict, List, Optional, Type, Union, Callable

try:
    from pydantic import BaseModel, ValidationError
except ImportError:
    BaseModel = None
    ValidationError = None

from ._dapil import App as _App, setup_logging
from .exceptions import HTTPException
from .responses import Response, StreamingResponse
from .requests import Request
from .middleware import BaseHTTPMiddleware as BaseMiddleware
from .routing import APIRouter

def _wrap_handler(handler: Callable):
    sig = inspect.signature(handler)
    params = sig.parameters
    is_async = inspect.iscoroutinefunction(handler)

    @functools.wraps(handler)
    async def wrapper(request: Request, **kwargs):
        try:
            call_args = {}
            for name, param in params.items():
                if param.annotation is Request or name == "request":
                    call_args[name] = request
                elif name in kwargs:
                    # Path param
                    val = kwargs[name]
                    if param.annotation is int:
                        try:
                            call_args[name] = int(val)
                        except ValueError:
                            raise HTTPException(status_code=400, detail=f"Path parameter '{name}' must be an integer")
                    else:
                        call_args[name] = val
                elif name in request.query_params:
                    # Query param
                    val = request.query_params[name]
                    if param.annotation is int:
                        try:
                            call_args[name] = int(val)
                        except ValueError:
                            raise HTTPException(status_code=400, detail=f"Query parameter '{name}' must be an integer")
                    else:
                        call_args[name] = val
                elif BaseModel and inspect.isclass(param.annotation) and issubclass(param.annotation, BaseModel):
                    # Pydantic model from body
                    try:
                        body_json = await request.json()
                    except Exception:
                        raise HTTPException(status_code=400, detail="Invalid JSON body")
                    
                    try:
                        call_args[name] = param.annotation(**body_json)
                    except ValidationError as e:
                        raise HTTPException(status_code=422, detail=e.errors())
                    except Exception as e:
                        raise HTTPException(status_code=422, detail=str(e))
            
            if is_async:
                res = await handler(**call_args)
            else:
                res = handler(**call_args)

            if isinstance(res, (dict, list)):
                import json
                return Response(json.dumps(res), status_code=200, headers={"Content-Type": "application/json"})
            return res
        except HTTPException as e:
            return Response(e.detail if isinstance(e.detail, (str, bytes)) else str(e.detail), status_code=e.status_code, headers={"Content-Type": "text/plain"})
        except Exception as e:
            import json
            error_detail = str(e)
            return Response(json.dumps({"detail": error_detail}), status_code=500, headers={"Content-Type": "application/json"})

    return wrapper

class App:
    def __init__(self):
        self._app = _App()
        self.middlewares = []
        
    def route(self, method: str, path: str):
        def decorator(func: Callable):
            wrapped = _wrap_handler(func)
            # Use the wrapper but keep original name for logging/debugging if needed
            self._app.route(method, path, wrapped)
            return func
        return decorator
        
    def add_middleware(self, middleware_class: type, **options: Any):
        self._app.add_middleware_instance(middleware_class(self, **options))

    def include_router(self, router: APIRouter):
        for method, path, handler in router.routes:
            self._app.route(method, path, _wrap_handler(handler))

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
