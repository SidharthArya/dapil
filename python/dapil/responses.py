from typing import Any, Dict, Optional, Union, Iterable, AsyncIterable

class Response:
    def __init__(
        self,
        content: Any = None,
        status_code: int = 200,
        headers: Optional[Dict[str, str]] = None,
        media_type: Optional[str] = None,
    ) -> None:
        self.content = content
        self.status_code = status_code
        self.headers = headers or {}
        self.media_type = media_type
        if media_type and "content-type" not in [k.lower() for k in self.headers.keys()]:
            self.headers["content-type"] = media_type

class StreamingResponse(Response):
    def __init__(
        self,
        content: Union[Iterable[Any], AsyncIterable[Any]],
        status_code: int = 200,
        headers: Optional[Dict[str, str]] = None,
        media_type: Optional[str] = None,
    ) -> None:
        super().__init__(content, status_code, headers, media_type)

class HTMLResponse(Response):
    def __init__(
        self,
        content: Any = None,
        status_code: int = 200,
        headers: Optional[Dict[str, str]] = None,
    ) -> None:
        super().__init__(content, status_code, headers, media_type="text/html")

import json
from datetime import datetime, date
from uuid import UUID
from enum import Enum

try:
    import orjson
except ImportError:
    orjson = None

def orjson_default(obj: Any) -> Any:
    if isinstance(obj, (datetime, date)):
        return obj.isoformat()
    if isinstance(obj, UUID):
        return str(obj)
    if isinstance(obj, Enum):
        return obj.value
    if hasattr(obj, "dict") and callable(obj.dict):
        return obj.dict()
    if hasattr(obj, "model_dump") and callable(obj.model_dump):
        return obj.model_dump()
    raise TypeError(f"Object of type {obj.__class__.__name__} is not JSON serializable")

class DapilJSONEncoder(json.JSONEncoder):
    def default(self, obj: Any) -> Any:
        try:
            return orjson_default(obj)
        except TypeError:
            return super().default(obj)

def json_dumps(obj: Any, **kwargs: Any) -> Union[str, bytes]:
    if orjson:
        # orjson.dumps returns bytes
        return orjson.dumps(obj, default=orjson_default)
    return json.dumps(obj, cls=DapilJSONEncoder, **kwargs)

class JSONResponse(Response):
    def __init__(
        self,
        content: Any = None,
        status_code: int = 200,
        headers: Optional[Dict[str, str]] = None,
    ) -> None:
        if not isinstance(content, (str, bytes)):
            content = json_dumps(content)
        super().__init__(content, status_code, headers, media_type="application/json")
