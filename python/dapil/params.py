from typing import Any, Optional

class Param:
    def __init__(
        self,
        default: Any = ...,
        *,
        alias: Optional[str] = None,
        title: Optional[str] = None,
        description: Optional[str] = None,
        **extra: Any,
    ):
        self.default = default
        self.alias = alias
        self.title = title
        self.description = description
        self.extra = extra

class Header(Param):
    pass

class Cookie(Param):
    pass

class Form(Param):
    pass

class File(Param):
    pass
