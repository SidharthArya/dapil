import json
from typing import Any, Dict, Optional
from urllib.parse import parse_qs

class State:
    def __init__(self, state: Optional[Dict[str, Any]] = None):
        if state is None:
            state = {}
        super().__setattr__("_state", state)

    def __setattr__(self, name: str, value: Any) -> None:
        self._state[name] = value

    def __getattr__(self, name: str) -> Any:
        try:
            return self._state[name]
        except KeyError:
            raise AttributeError(f"'{type(self).__name__}' object has no attribute '{name}'")

    def __len__(self) -> int:
        return len(self._state)

class Request:
    def __init__(self, scope: Dict[str, Any], receive: Optional[Any] = None):
        self.scope = scope
        self._receive = receive
        self.state = State()
        self._body = scope.get("_body", b"")
        
    @property
    def method(self) -> str:
        return self.scope["method"]

    @property
    def query_params(self) -> Dict[str, Any]:
        if "_query_params" not in self.__dict__:
            query_string = self.scope.get("query_string", b"").decode("latin-1")
            self._query_params = {k: v[0] if len(v) == 1 else v for k, v in parse_qs(query_string).items()}
        return self._query_params

    @property
    def headers(self) -> Dict[str, str]:
        if "_headers" not in self.__dict__:
            self._headers = {k.decode("latin-1") if isinstance(k, bytes) else k: v.decode("latin-1") if isinstance(v, bytes) else v for k, v in self.scope.get("headers", [])}
        return self._headers

    async def body(self) -> bytes:
        return self._body

    async def json(self) -> Any:
        return json.loads(self._body)

    @property
    def url(self):
        # Mini URL object for compatibility
        class URL:
            def __init__(self, scope):
                self.path = scope.get("path", "")
                self.query = scope.get("query_string", b"").decode("latin-1")
        return URL(self.scope)
