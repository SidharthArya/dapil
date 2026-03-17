import inspect
import functools
import asyncio
from typing import Any, Dict, List, Optional, Type, Union, Callable

try:
    from pydantic import BaseModel, ValidationError
except ImportError:
    BaseModel = None
    ValidationError = None

from ._dapil import App as _App, setup_logging, Request
from .exceptions import HTTPException
from .responses import Response, StreamingResponse, HTMLResponse, JSONResponse, json_dumps
# from .requests import Request  # Replaced by native Request
from .middleware import BaseHTTPMiddleware as BaseMiddleware
from .routing import APIRouter
from .openapi import get_openapi, get_swagger_ui_html
from .depends import Depends

def _build_route_schema(handler: Callable, path: str) -> List[Dict[str, str]]:
    schema = []
    seen = set()
    
    def _extract(func):
        if func in seen: return
        seen.add(func)
        sig = inspect.signature(func)
        for name, param in sig.parameters.items():
            if name == "request" or (hasattr(param.annotation, "__name__") and param.annotation.__name__ == "Request"):
                continue
            
            if isinstance(param.default, Depends):
                if param.default.dependency:
                    _extract(param.default.dependency)
                continue
                
            if BaseModel and inspect.isclass(param.annotation) and issubclass(param.annotation, BaseModel):
                schema.append({"name": name, "source": "body", "type": "json"})
            else:
                is_path_param = f"{{{name}}}" in path
                source = "path" if is_path_param else "query"
                type_str = "str"
                if param.annotation is int:
                    type_str = "int"
                elif param.annotation is float:
                    type_str = "float"
                elif param.annotation is bool:
                    type_str = "bool"
                if not any(s["name"] == name for s in schema):
                    schema.append({"name": name, "source": source, "type": type_str})
                
    _extract(handler)
    return schema

async def _resolve_params(handler: Callable, request: Request, kwargs: Dict[str, Any], cache: Dict[Any, Any]):
    sig = inspect.signature(handler)
    call_args = {}
    
    for name, param in sig.parameters.items():
        if isinstance(param.default, Depends):
            dependency = param.default.dependency
            if dependency is None:
                continue
            
            if param.default.use_cache and dependency in cache:
                call_args[name] = cache[dependency]
                continue
                
            dep_kwargs = await _resolve_params(dependency, request, kwargs, cache)
            if inspect.iscoroutinefunction(dependency):
                res = await dependency(**dep_kwargs)
            else:
                res = dependency(**dep_kwargs)
                
            if param.default.use_cache:
                cache[dependency] = res
            call_args[name] = res
            continue
            
        elif param.annotation is Request or name == "request":
            call_args[name] = request
        elif name in kwargs:
            val = kwargs[name]
            if BaseModel and inspect.isclass(param.annotation) and issubclass(param.annotation, BaseModel):
                if isinstance(val, dict):
                    try:
                        call_args[name] = param.annotation(**val)
                    except ValidationError as e:
                        raise HTTPException(status_code=422, detail=e.errors())
                else:
                    call_args[name] = val
            else:
                call_args[name] = val
        else:
            if param.default is not inspect.Parameter.empty:
                call_args[name] = param.default
            else:
                raise HTTPException(status_code=400, detail=f"Missing parameter '{name}'")
    return call_args

def _wrap_handler(handler: Callable):
    is_async = inspect.iscoroutinefunction(handler)
    sig = inspect.signature(handler)
    
    # Pre-calculate dependency and parameter mapping
    params_info = []
    for name, param in sig.parameters.items():
        is_request = (param.annotation is Request or name == "request")
        dependency = param.default.dependency if isinstance(param.default, Depends) else None
        params_info.append({
            "name": name,
            "is_request": is_request,
            "dependency": dependency,
            "use_cache": getattr(param.default, "use_cache", True) if dependency else False,
            "default": param.default if not is_request and not dependency and param.default is not inspect.Parameter.empty else None,
            "has_default": not is_request and not dependency and param.default is not inspect.Parameter.empty,
            "annotation": param.annotation
        })

    @functools.wraps(handler)
    async def wrapper(request: Request, **kwargs):
        try:
            cache = {}
            call_args = {}
            
            for p in params_info:
                name = p["name"]
                if p["dependency"]:
                    dependency = p["dependency"]
                    if p["use_cache"] and dependency in cache:
                        call_args[name] = cache[dependency]
                    else:
                        # Recursive resolution still needed for dependencies
                        dep_kwargs = await _resolve_params(dependency, request, kwargs, cache)
                        if inspect.iscoroutinefunction(dependency):
                            res = await dependency(**dep_kwargs)
                        else:
                            res = dependency(**dep_kwargs)
                        if p["use_cache"]:
                            cache[dependency] = res
                        call_args[name] = res
                elif p["is_request"]:
                    call_args[name] = request
                elif name in kwargs:
                    val = kwargs[name]
                    if BaseModel and inspect.isclass(p["annotation"]) and issubclass(p["annotation"], BaseModel):
                        if isinstance(val, dict):
                            try:
                                call_args[name] = p["annotation"](**val)
                            except ValidationError as e:
                                raise HTTPException(status_code=422, detail=e.errors())
                        else:
                            call_args[name] = val
                    else:
                        call_args[name] = val
                elif p["has_default"]:
                    call_args[name] = p["default"]
                else:
                    raise HTTPException(status_code=400, detail=f"Missing parameter '{name}'")
            
            if is_async:
                res = await handler(**call_args)
            else:
                res = handler(**call_args)

            if isinstance(res, (dict, list)):
                return Response(json_dumps(res), status_code=200, headers={"Content-Type": "application/json"})
            return res
        except HTTPException as e:
            return Response(e.detail if isinstance(e.detail, (str, bytes)) else str(e.detail), status_code=e.status_code, headers={"Content-Type": "text/plain"})
        except Exception as e:
            error_detail = str(e)
            return Response(json_dumps({"detail": error_detail}), status_code=500, headers={"Content-Type": "application/json"})

    return wrapper

class App:
    def __init__(
        self,
        title: str = "Dapil API",
        version: str = "0.1.0",
        openapi_url: Optional[str] = "/openapi.json",
        docs_url: Optional[str] = "/docs",
        description: Optional[str] = None,
    ):
        self.title = title
        self.version = version
        self.openapi_url = openapi_url
        self.docs_url = docs_url
        self.description = description
        
        self._app = _App()
        self.middlewares = []
        self.routes = []
        
        self._setup_docs()
        
    def _setup_docs(self):
        if self.openapi_url:
            @self.get(self.openapi_url)
            def openapi_schema():
                return JSONResponse(
                    get_openapi(
                        title=self.title,
                        version=self.version,
                        routes=self.routes,
                        description=self.description,
                    )
                )

        if self.docs_url and self.openapi_url:
            @self.get(self.docs_url)
            def swagger_ui_html():
                return HTMLResponse(
                    get_swagger_ui_html(
                        openapi_url=self.openapi_url,
                        title=f"{self.title} - Swagger UI",
                    )
                )
        
    def route(self, method: str, path: str):
        def decorator(func: Callable):
            schema = _build_route_schema(func, path)
            wrapped = _wrap_handler(func)
            self._app.route(method, path, wrapped, schema)
            self.routes.append({
                "method": method,
                "path": path,
                "func": func,
            })
            return func
        return decorator
        
    def add_middleware(self, middleware_class: type, **options: Any):
        self._app.add_middleware_instance(middleware_class(self, **options))

    def include_router(self, router: APIRouter, prefix: str = ""):
        for method, path, handler in router.routes:
            full_path = prefix + path
            if not full_path.startswith("/"):
                full_path = "/" + full_path
            schema = _build_route_schema(handler, full_path)
            self.routes.append({
                "method": method,
                "path": full_path,
                "func": handler,
            })
            self._app.route(method, full_path, _wrap_handler(handler), schema)

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
