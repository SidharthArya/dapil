from typing import Any, Optional

class Param:
    def __init__(
        self,
        default: Any = ...,
        *,
        alias: Optional[str] = None,
        title: Optional[str] = None,
        description: Optional[str] = None,
        # Numeric validation
        gt: Optional[float] = None,
        ge: Optional[float] = None,
        lt: Optional[float] = None,
        le: Optional[float] = None,
        multiple_of: Optional[float] = None,
        # String validation
        min_length: Optional[int] = None,
        max_length: Optional[int] = None,
        pattern: Optional[str] = None,
        **extra: Any,
    ):
        self.default = default
        self.alias = alias
        self.title = title
        self.description = description
        self.gt = gt
        self.ge = ge
        self.lt = lt
        self.le = le
        self.multiple_of = multiple_of
        self.min_length = min_length
        self.max_length = max_length
        self.pattern = pattern
        self.extra = extra

class Path(Param):
    pass

class Query(Param):
    pass

class Body(Param):
    pass

class Header(Param):
    pass

class Cookie(Param):
    pass

class Form(Param):
    pass

class File(Param):
    pass
