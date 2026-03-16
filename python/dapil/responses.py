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

class JSONResponse(Response):
    def __init__(
        self,
        content: Any = None,
        status_code: int = 200,
        headers: Optional[Dict[str, str]] = None,
    ) -> None:
        import json
        if not isinstance(content, (str, bytes)):
            content = json.dumps(content)
        super().__init__(content, status_code, headers, media_type="application/json")
