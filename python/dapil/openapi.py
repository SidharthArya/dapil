import inspect
from typing import Any, Callable, Dict, List, Optional, Type
try:
    from pydantic import BaseModel, BaseModel as _BaseModel
except ImportError:
    BaseModel = None

from .depends import Depends


def _get_route_parameters(func: Callable, path: str, components_schemas: Dict[str, Any], seen_deps: Optional[set] = None):
    if seen_deps is None:
        seen_deps = set()
    
    parameters = []
    request_body = None
    
    if func in seen_deps:
        return parameters, request_body
    seen_deps.add(func)

    sig = inspect.signature(func)
    for name, param in sig.parameters.items():
        if name == "request" or (hasattr(param.annotation, "__name__") and param.annotation.__name__ == "Request"):
            continue

        if isinstance(param.default, Depends):
            if param.default.dependency:
                sub_params, sub_body = _get_route_parameters(param.default.dependency, path, components_schemas, seen_deps)
                parameters.extend(sub_params)
                if sub_body:
                    request_body = sub_body
            continue

        # Check if it's a Pydantic model (request body)
        if BaseModel and inspect.isclass(param.annotation) and issubclass(param.annotation, BaseModel):
            model_name = param.annotation.__name__
            
            # Add to components.schemas if not exists
            if model_name not in components_schemas:
                if hasattr(param.annotation, "model_json_schema"):
                    components_schemas[model_name] = param.annotation.model_json_schema()
                else:
                    components_schemas[model_name] = param.annotation.schema()

            request_body = {
                "content": {
                    "application/json": {
                        "schema": {"$ref": f"#/components/schemas/{model_name}"}
                    }
                },
                "required": True,
            }
        else:
            # Path or Query parameter
            # Weak heuristic: if name is in the path format {name}, it's a path param.
            # Since we don't have a rigid router parsing here, we check path string.
            is_path_param = f"{{{name}}}" in path
            param_in = "path" if is_path_param else "query"
            
            schema = {"type": "string"}
            if param.annotation is int:
                schema["type"] = "integer"
            elif param.annotation is bool:
                schema["type"] = "boolean"
            elif param.annotation is float:
                schema["type"] = "number"

            param_dict = {
                "name": name,
                "in": param_in,
                "required": param.default == inspect.Parameter.empty,
                "schema": schema,
            }
            # Only add to parameters if not already in there to avoid duplicates from multi-depends
            if not any(p["name"] == name and p["in"] == param_in for p in parameters):
                parameters.append(param_dict)

    return parameters, request_body

def get_openapi(
    title: str,
    version: str,
    routes: List[Dict[str, Any]],
    openapi_version: str = "3.1.0",
    description: Optional[str] = None,
) -> Dict[str, Any]:
    """
    Generate an OpenAPI schema based on the registered routes.
    """
    info = {"title": title, "version": version}
    if description:
        info["description"] = description

    output: Dict[str, Any] = {
        "openapi": openapi_version,
        "info": info,
        "paths": {},
        "components": {"schemas": {}},
    }

    components_schemas = output["components"]["schemas"]

    for route in routes:
        path = route["path"]
        method = route["method"].lower()
        func = route["func"]

        # Convert /items/{item_id} to valid openapi if needed, though usually the same
        openapi_path = path

        if openapi_path not in output["paths"]:
            output["paths"][openapi_path] = {}

        operation: Dict[str, Any] = {
            "summary": func.__name__.replace("_", " ").title(),
            "operationId": f"{func.__name__}_{method}",
            "responses": {
                "200": {
                    "description": "Successful Response",
                    "content": {"application/json": {"schema": {}}}
                }
            }
        }
        
        if func.__doc__:
            operation["description"] = inspect.cleandoc(func.__doc__)

        parameters, request_body = _get_route_parameters(func, path, components_schemas)

        if parameters:
            operation["parameters"] = parameters
        if request_body:
            operation["requestBody"] = request_body
            
        # Handle response_model
        response_model = route.get("response_model")
        if response_model:
            model_name = None
            if BaseModel and inspect.isclass(response_model) and issubclass(response_model, BaseModel):
                model_name = response_model.__name__
                if model_name not in components_schemas:
                    if hasattr(response_model, "model_json_schema"):
                        components_schemas[model_name] = response_model.model_json_schema()
                    else:
                        components_schemas[model_name] = response_model.schema()
                schema_ref = {"$ref": f"#/components/schemas/{model_name}"}
            else:
                # Handle List[User] etc.
                try:
                    from pydantic import TypeAdapter
                    adapter = TypeAdapter(response_model)
                    # This is more complex since it might return integrated definitions
                    res_schema = adapter.json_schema()
                    if "$defs" in res_schema:
                        for k, v in res_schema["$defs"].items():
                            if k not in components_schemas:
                                components_schemas[k] = v
                        del res_schema["$defs"]
                    schema_ref = res_schema
                except:
                    schema_ref = {}

            if schema_ref:
                operation["responses"]["200"]["content"]["application/json"]["schema"] = schema_ref

        output["paths"][openapi_path][method] = operation

    return output


def get_swagger_ui_html(
    *,
    openapi_url: str,
    title: str,
    swagger_js_url: str = "https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui-bundle.js",
    swagger_css_url: str = "https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui.css",
) -> str:
    """
    Generate the HTML for Swagger UI.
    """
    html = f"""
    <!DOCTYPE html>
    <html>
    <head>
    <link type="text/css" rel="stylesheet" href="{swagger_css_url}">
    <title>{title}</title>
    </head>
    <body>
    <div id="swagger-ui">
    </div>
    <script src="{swagger_js_url}"></script>
    <script>
    const ui = SwaggerUIBundle({{
        url: '{openapi_url}',
        dom_id: '#swagger-ui',
        presets: [
        SwaggerUIBundle.presets.apis,
        SwaggerUIBundle.SwaggerUIStandalonePreset
        ],
        layout: "BaseLayout",
        deepLinking: true,
        showExtensions: true,
        showCommonExtensions: true
    }})
    </script>
    </body>
    </html>
    """
    return html
