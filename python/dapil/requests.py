from typing import Any, Dict, Optional

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
        
    @property
    def method(self) -> str:
        return self.scope["method"]

    @property
    def headers(self) -> Dict[str, str]:
        if "_headers" not in self.__dict__:
            self._headers = {k.decode("latin-1"): v.decode("latin-1") for k, v in self.scope.get("headers", [])}
        return self._headers

    @property
    def url(self):
        # Mini URL object for compatibility
        class URL:
            def __init__(self, scope):
                self.path = scope.get("path", "")
        return URL(self.scope)
