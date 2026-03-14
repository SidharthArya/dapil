
from typing import Callable, List, Tuple

class APIRouter:
    def __init__(self, prefix: str = ""):
        self.prefix = prefix
        self.routes: List[Tuple[str, str, Callable]] = []

    def route(self, method: str, path: str):
        def decorator(func: Callable):
            # Prepend router's own prefix if any
            full_path = self.prefix + path
            # Ensure path starts with /
            if not full_path.startswith("/"):
                full_path = "/" + full_path
            # Normalize trailing slashes if needed, but Axum is usually strict
            self.routes.append((method.to_uppercase() if hasattr(method, "to_uppercase") else method.upper(), full_path, func))
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
