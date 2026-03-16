from typing import Callable, Any
from .requests import Request
from .responses import Response

RequestResponseEndpoint = Callable[[Request], Any]

class BaseHTTPMiddleware:
    def __init__(self, app: Any):
        self.app = app

    async def __call__(self, request: Request, call_next: RequestResponseEndpoint) -> Response:
        return await self.dispatch(request, call_next)

    async def dispatch(self, request: Request, call_next: RequestResponseEndpoint) -> Response:
        raise NotImplementedError()
