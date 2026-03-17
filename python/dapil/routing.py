from typing import Callable, List, Tuple, Any, Optional

class APIRouter:
    def __init__(self, prefix: str = ""):
        self.prefix = prefix
        self.routes: List[dict] = []

    def route(self, method: str, path: str, response_model: Optional[Any] = None):
        def decorator(func: Callable):
            # Prepend router's own prefix if any
            full_path = self.prefix + path
            # Ensure path starts with /
            if not full_path.startswith("/"):
                full_path = "/" + full_path
            
            self.routes.append({
                "method": method.upper(),
                "path": full_path,
                "handler": func,
                "options": {"response_model": response_model}
            })
            return func
        return decorator

    def get(self, path: str, response_model: Optional[Any] = None):
        return self.route("GET", path, response_model=response_model)

    def post(self, path: str, response_model: Optional[Any] = None):
        return self.route("POST", path, response_model=response_model)

    def put(self, path: str, response_model: Optional[Any] = None):
        return self.route("PUT", path, response_model=response_model)

    def delete(self, path: str, response_model: Optional[Any] = None):
        return self.route("DELETE", path, response_model=response_model)
