mod methods;

use pyo3::prelude::*;
use axum::{
    routing,
    Router,
};
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tracing::info;
use pyo3::PyObject;
// use tower_http::trace::TraceLayer;
use std::thread;
use tokio::sync::{oneshot, mpsc};
use axum::{
    response::{IntoResponse, Response},
    http::{StatusCode, HeaderMap, HeaderName, HeaderValue},
    extract::{Request as AxumRequest, Path},
    body::Body,
};
use pyo3::types::{PyDict, PyList};
use std::sync::Arc;
use tokio_stream::StreamExt;
use form_urlencoded;
use serde_json;

enum BodyData {
    Bytes(Vec<u8>),
    Stream(mpsc::Receiver<Vec<u8>>),
}

struct ResponseData {
    status: u16,
    body: BodyData,
    headers: Vec<(String, String)>,
}

impl IntoResponse for ResponseData {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let mut headers = HeaderMap::new();
        for (k, v) in self.headers {
            if let (Ok(name), Ok(value)) = (HeaderName::from_bytes(k.as_bytes()), HeaderValue::from_str(&v)) {
                headers.insert(name, value);
            }
        }

        match self.body {
            BodyData::Bytes(bytes) => (status, headers, bytes).into_response(),
            BodyData::Stream(rx) => {
                let stream = tokio_stream::wrappers::ReceiverStream::new(rx)
                    .map(|chunk| Ok::<_, std::convert::Infallible>(axum::body::Bytes::from(chunk)));
                let body = Body::from_stream(stream);
                (status, headers, body).into_response()
            }
        }
    }
}

#[pyclass]
struct PyCallNext {
    tx: mpsc::Sender<(Py<PyAny>, oneshot::Sender<ResponseData>)>,
}

#[derive(Clone)]
struct PyMiddleware(Arc<Py<PyAny>>);
unsafe impl Send for PyMiddleware {}
unsafe impl Sync for PyMiddleware {}

#[pymethods]
impl PyCallNext {
    #[pyo3(signature = (request))]
    async fn __call__(&self, request: Py<PyAny>) -> PyResult<Py<PyAny>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        
        // Send request to the next step
        self.tx.send((request, resp_tx)).await
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("call_next handler channel closed"))?;
        
        // Wait for result
        let resp_data = resp_rx.await
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("call_next response channel closed"))?;
        
        Python::with_gil(|py| {
            let dapil = py.import("dapil")?;
            let response_cls = dapil.getattr("Response")?;
            
            let body = match resp_data.body {
                BodyData::Bytes(b) => pyo3::types::PyBytes::new(py, &b).into_any().unbind(),
                BodyData::Stream(_) => py.None(),
            };
            
            let headers_dict = PyDict::new(py);
            for (k, v) in resp_data.headers {
                headers_dict.set_item(k, v)?;
            }
            
            response_cls.call1((body, resp_data.status, headers_dict)).map(|b| b.unbind())
        })
    }
}
#[derive(Debug, Clone)]
struct PyHandler(Arc<Py<PyAny>>);
unsafe impl Send for PyHandler {}
unsafe impl Sync for PyHandler {}

// Custom Clone removed as Arc provides it

#[derive(Clone, Debug)]
struct ParamDef {
    name: String,
    source: String,
    param_type: String,
    gt: Option<f64>,
    ge: Option<f64>,
    lt: Option<f64>,
    le: Option<f64>,
    min_length: Option<usize>,
    max_length: Option<usize>,
    pattern: Option<String>,
}

fn json_to_py(py: Python, val: &serde_json::Value) -> PyObject {
    use pyo3::ToPyObject;
    match val {
        serde_json::Value::Null => py.None(),
        serde_json::Value::Bool(b) => b.to_object(py),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() { i.to_object(py) }
            else if let Some(f) = n.as_f64() { f.to_object(py) }
            else { py.None() }
        },
        serde_json::Value::String(s) => s.to_object(py),
        serde_json::Value::Array(arr) => {
            let list = pyo3::types::PyList::empty(py);
            for item in arr {
                let _ = list.append(json_to_py(py, item));
            }
            list.into()
        },
        serde_json::Value::Object(obj) => {
            let dict = pyo3::types::PyDict::new(py);
            for (k, v) in obj {
                let _ = dict.set_item(k, json_to_py(py, v));
            }
            dict.into()
        }
    }
}

fn py_response_to_axum(py: Python, py_resp: Option<&Bound<'_, PyAny>>) -> axum::response::Response {
    let py_resp = match py_resp {
        Some(r) => r,
        None => {
            return axum::response::Response::builder().status(500).body("Internal Error".into()).unwrap();
        }
    };
    
    if let Ok(s) = py_resp.extract::<String>() {
        return axum::response::Response::builder()
            .status(200)
            .header("content-type", "text/plain")
            .body(axum::body::Body::from(s))
            .unwrap();
    }
    
    if let Ok(b) = py_resp.extract::<Vec<u8>>() {
        return axum::response::Response::builder()
            .status(200)
            .header("content-type", "application/octet-stream")
            .body(axum::body::Body::from(b))
            .unwrap();
    }

    if let Ok(content) = py_resp.getattr("content") {
        let status = py_resp.getattr("status_code").and_then(|s| s.extract::<u16>()).unwrap_or(200);
        let mut builder = axum::response::Response::builder().status(status);
        if let Ok(headers_dict) = py_resp.getattr("headers") {
            if let Ok(dict) = headers_dict.downcast::<PyDict>() {
                for (k, v) in dict {
                    if let (Ok(ks), Ok(vs)) = (k.extract::<String>(), v.extract::<String>()) {
                        builder = builder.header(ks, vs);
                    }
                }
            }
        }
        
        let is_streaming = !content.is_instance_of::<pyo3::types::PyString>() && 
                           !content.is_instance_of::<pyo3::types::PyBytes>() && 
                           content.try_iter().is_ok();
                           
        if is_streaming {
            if let Ok(it) = content.try_iter() {
                let mut full_body = Vec::new();
                for chunk_res in it {
                    if let Ok(chunk_obj) = chunk_res {
                        if let Ok(b) = chunk_obj.extract::<Vec<u8>>() {
                            full_body.extend(b);
                        } else if let Ok(s) = chunk_obj.extract::<String>() {
                            full_body.extend(s.into_bytes());
                        }
                    }
                }
                return builder.body(axum::body::Body::from(full_body)).unwrap();
            }
        } else {
            let body = if let Ok(s) = content.extract::<String>() {
                axum::body::Body::from(s)
            } else if let Ok(b) = content.extract::<Vec<u8>>() {
                axum::body::Body::from(b)
            } else {
                axum::body::Body::from(vec![])
            };
            return builder.body(body).unwrap();
        }
    }
    
    axum::response::Response::builder().status(500).body("Unsupported result".into()).unwrap()
}

#[pyclass(module = "dapil._dapil")]
pub struct Request {
    #[pyo3(get)]
    pub method: String,
    #[pyo3(get)]
    pub path: String,
    #[pyo3(get)]
    pub query_string: String,
    pub headers: HeaderMap,
    pub body_bytes: axum::body::Bytes,
}

#[pymethods]
impl Request {
    #[getter]
    fn headers<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyDict>> {
        let dict = PyDict::new(py);
        for (name, value) in &self.headers {
            if let Ok(v) = value.to_str() {
                dict.set_item(name.as_str(), v)?;
            }
        }
        Ok(dict)
    }

    #[getter]
    fn query_params<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyDict>> {
        let dict = PyDict::new(py);
        for (k, v) in form_urlencoded::parse(self.query_string.as_bytes()) {
            dict.set_item(k, v)?;
        }
        Ok(dict)
    }

    #[getter]
    fn url(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        dict.set_item("path", &self.path)?;
        dict.set_item("query", &self.query_string)?;
        Ok(dict.into())
    }

    fn body<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, pyo3::types::PyBytes>> {
        Ok(pyo3::types::PyBytes::new(py, &self.body_bytes))
    }

    fn json(&self, py: Python) -> PyResult<PyObject> {
        let val: serde_json::Value = serde_json::from_slice(&self.body_bytes)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(json_to_py(py, &val))
    }
}

fn handle_py_error(py: Python, e: pyo3::PyErr) -> axum::response::Response {
    if e.is_instance_of::<pyo3::exceptions::PyException>(py) {
        let val = e.value(py);
        let status_code = val.getattr("status_code").and_then(|s| s.extract::<u16>()).unwrap_or(500);
        let detail = val.getattr("detail").and_then(|d| d.extract::<String>()).unwrap_or_else(|_| "Internal Error".to_string());
        axum::response::Response::builder()
            .status(status_code)
            .header("content-type", "text/plain")
            .body(detail.into())
            .unwrap()
    } else {
        e.print(py);
        axum::response::Response::builder()
            .status(500)
            .header("content-type", "text/plain")
            .body("Internal Server Error".into())
            .unwrap()
    }
}

#[pyclass]
struct App {
    host: String,
    port: u16,
    workers: usize,
    // Store routes as (path, method) -> handler
    routes: Vec<(String, String, PyHandler, PyObject)>,
    middlewares: Vec<PyMiddleware>,
}

#[pymethods]
impl App {
    #[new]
    fn new() -> Self {
        // Automatically initialize logging bridge if not already done
        let _ = pyo3_log::try_init();
        App {
            host: "0.0.0.0".to_string(),
            port: 8080,
            workers: 1,
            routes: Vec::new(),
            middlewares: Vec::new(),
        }
    }

    fn add_middleware_instance(&mut self, instance: Py<PyAny>) {
        self.middlewares.push(PyMiddleware(Arc::new(instance)));
    }

    fn set_host(&mut self, host: &str) {
        self.host = String::from(host);
    }

    fn set_port(&mut self, port: u16) {
        self.port = port;
    }

    fn set_workers(&mut self, workers: usize) {
        self.workers = workers;
    }

    fn route(&mut self, method: String, path: String, handler: PyObject, schema: PyObject) {
        self.routes.push((path, method.to_uppercase(), PyHandler(Arc::new(handler)), schema));
    }

    fn get(&mut self, path: String, handler: PyObject, schema: PyObject) {
        self.route("GET".to_string(), path, handler, schema);
    }

    fn post(&mut self, path: String, handler: PyObject, schema: PyObject) {
        self.route("POST".to_string(), path, handler, schema);
    }

    fn put(&mut self, path: String, handler: PyObject, schema: PyObject) {
        self.route("PUT".to_string(), path, handler, schema);
    }

    fn delete(&mut self, path: String, handler: PyObject, schema: PyObject) {
        self.route("DELETE".to_string(), path, handler, schema);
    }

    fn serve_default(&mut self, py: Python<'_>) -> PyResult<()> {
        self.serve(py)
    }

    fn serve(&mut self, py: Python<'_>) -> PyResult<()> {
        if self.workers > 1 {
            self.serve_multi(py)
        } else {
            self.serve_single(py)
        }
    }
}

impl App {
    fn serve_single(&mut self, py: Python<'_>) -> PyResult<()> {
        self.setup_and_run(py)?;
        Ok(())
    }

    fn serve_multi(&mut self, _py: Python<'_>) -> PyResult<()> {
        use nix::unistd::{fork, ForkResult};
        use nix::sys::wait::wait;

        info!("Starting Dapil with {} workers", self.workers);

        for i in 0..self.workers {
            match unsafe { fork() } {
                Ok(ForkResult::Parent { child }) => {
                    info!("Spawned worker {} (PID: {})", i, child);
                }
                Ok(ForkResult::Child) => {
                    let _ = self.serve_worker();
                    std::process::exit(0);
                }
                Err(_) => panic!("Fork failed"),
            }
        }

        for _ in 0..self.workers {
            let _ = wait();
        }
        Ok(())
    }

    fn serve_worker(&mut self) -> PyResult<()> {
        Python::with_gil(|py| self.setup_and_run(py))?;
        Ok(())
    }

    pub fn setup_and_run(&mut self, py: Python) -> PyResult<()> {
        // Copy configs to move into Tokio task
        let routes_copy: Vec<(String, String, PyHandler, PyObject)> = self.routes.iter()
            .map(|(p, m, h, s)| (p.clone(), m.clone(), h.clone(), s.clone_ref(py)))
            .collect();
            
        let middlewares_copy: Vec<Arc<PyObject>> = self.middlewares.iter()
            .map(|m| Arc::new(m.0.clone_ref(py)))
            .collect();
            
        let host = self.host.clone();
        let port = self.port;

        pyo3_async_runtimes::tokio::run(py, async move {
            let locals = Python::with_gil(|py| pyo3_async_runtimes::tokio::get_current_locals(py).unwrap());
            let locals_arc = Arc::new(locals);
            
            let mut router = Router::new();

            let is_routes_empty = routes_copy.is_empty();
            // Apply Python routes
            for (path, method, handler, schema) in routes_copy {
                let handler = handler.clone();
                let path = path.clone();
                let locals_arc = locals_arc.clone();

                let is_async = Python::with_gil(|py| {
                    match py.import("inspect") {
                        Ok(inspect) => match inspect.call_method1("iscoroutinefunction", (handler.0.bind(py),)) {
                            Ok(res) => res.extract::<bool>().unwrap_or(false),
                            Err(_) => false,
                        },
                        Err(_) => false,
                    }
                });

                let mut params_schema: Vec<ParamDef> = Vec::new();
                let mut needs_body = false;
                let mut needs_query = false;
                Python::with_gil(|py| {
                    if let Ok(schema_list) = schema.bind(py).downcast::<PyList>() {
                        for item in schema_list {
                            if let Ok(dict) = item.downcast::<PyDict>() {
                                let name_res: PyResult<Option<Bound<PyAny>>> = dict.get_item("name");
                                let source_res: PyResult<Option<Bound<PyAny>>> = dict.get_item("source");
                                let type_res: PyResult<Option<Bound<PyAny>>> = dict.get_item("type");
                                
                                if let (Ok(Some(name_obj)), Ok(Some(source_obj)), Ok(Some(type_obj))) = (name_res, source_res, type_res) {
                                    let source = source_obj.extract::<String>().unwrap_or_default();
                                    if source == "body" || source == "form" || source == "file" {
                                        needs_body = true;
                                    } else if source == "query" {
                                        needs_query = true;
                                    }
                                    params_schema.push(ParamDef {
                                        name: name_obj.extract::<String>().unwrap_or_default(),
                                        source,
                                        param_type: type_obj.extract::<String>().unwrap_or_default(),
                                        gt: dict.get_item("gt").unwrap_or(None).and_then(|v| v.extract::<f64>().ok()),
                                        ge: dict.get_item("ge").unwrap_or(None).and_then(|v| v.extract::<f64>().ok()),
                                        lt: dict.get_item("lt").unwrap_or(None).and_then(|v| v.extract::<f64>().ok()),
                                        le: dict.get_item("le").unwrap_or(None).and_then(|v| v.extract::<f64>().ok()),
                                        min_length: dict.get_item("min_length").unwrap_or(None).and_then(|v| v.extract::<usize>().ok()),
                                        max_length: dict.get_item("max_length").unwrap_or(None).and_then(|v| v.extract::<usize>().ok()),
                                        pattern: dict.get_item("pattern").unwrap_or(None).and_then(|v| v.extract::<String>().ok()),
                                    });
                                }
                            }
                        }
                    }
                });

                let axum_path = path.replace('{', ":").replace('}', "");
                
                let run_handler = move |params: Path<std::collections::HashMap<String, String>>, axum_req: AxumRequest| {
                    let params_schema = params_schema.clone();
                    let params = params.0;
                    let h = handler.clone();
                    let locals_clone = locals_arc.clone();
                    async move {
                        let locals_for_scope = Python::with_gil(|py| locals_clone.clone_ref(py));
                        pyo3_async_runtimes::tokio::scope(locals_for_scope, async move {
                            let (parts, body) = axum_req.into_parts();
                            
                            let body_bytes = if needs_body {
                                axum::body::to_bytes(body, 10 * 1024 * 1024).await.unwrap_or_default()
                            } else {
                                axum::body::Bytes::new()
                            };

                            enum ExecResult {
                                Done(axum::response::Response),
                                Future(std::pin::Pin<Box<dyn std::future::Future<Output = PyResult<PyObject>> + Send>>),
                            }

                            // Pre-extract parameters that might need async or are just easier outside
                            let mut pre_extracted = std::collections::HashMap::new();
                            
                            if needs_body && !body_bytes.is_empty() {
                                if let Some(content_type) = parts.headers.get(axum::http::header::CONTENT_TYPE) {
                                    if let Ok(ct_str) = content_type.to_str() {
                                        if ct_str.starts_with("multipart/form-data") {
                                            if let Some(boundary) = ct_str.split("boundary=").nth(1) {
                                                let body_bytes_tmp = body_bytes.clone();
                                                let data_stream = futures_util::stream::once(async move { Ok::<_, std::io::Error>(body_bytes_tmp) });
                                                let mut multipart = multer::Multipart::new(data_stream, boundary);
                                                
                                                while let Ok(Some(field)) = multipart.next_field().await {
                                                    if let Some(name) = field.name().map(|s| s.to_string()) {
                                                        let filename = field.file_name().map(|s| s.to_string());
                                                        let content_type = field.content_type().map(|s| s.to_string());
                                                        let data = field.bytes().await.unwrap_or_default();
                                                        pre_extracted.insert(name, (filename, content_type, data));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            let result = Python::with_gil(|py| -> ExecResult {
                                let query_string = parts.uri.query().unwrap_or("").to_string();
                                let req_obj = crate::Request {
                                    method: parts.method.to_string(),
                                    path: parts.uri.path().to_string(),
                                    query_string: query_string.clone(),
                                    headers: parts.headers.clone(),
                                    body_bytes: body_bytes.clone(),
                                };
                                let py_req = Bound::new(py, req_obj).expect("failed to create Request");
                                
                                let kwargs = PyDict::new(py);
                                
                                let mut query_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
                                if needs_query && !query_string.is_empty() {
                                    for (k, v) in form_urlencoded::parse(query_string.as_bytes()).into_owned() {
                                        query_map.insert(k, v);
                                    }
                                }

                                for pdef in &params_schema {
                                    match pdef.source.as_str() {
                                        "path" => {
                                            if let Some(val) = params.get(&pdef.name) {
                                                match pdef.param_type.as_str() {
                                                    "int" => if let Ok(i) = val.parse::<i64>() { let _ = kwargs.set_item(&pdef.name, i); },
                                                    "float" => if let Ok(f) = val.parse::<f64>() { let _ = kwargs.set_item(&pdef.name, f); },
                                                    "bool" => if let Ok(b) = val.parse::<bool>() { let _ = kwargs.set_item(&pdef.name, b); },
                                                    _ => { let _ = kwargs.set_item(&pdef.name, val); }
                                                }
                                            }
                                        },
                                        "query" => {
                                            if let Some(val) = query_map.get(&pdef.name) {
                                                match pdef.param_type.as_str() {
                                                    "int" => if let Ok(i) = val.parse::<i64>() { let _ = kwargs.set_item(&pdef.name, i); },
                                                    "float" => if let Ok(f) = val.parse::<f64>() { let _ = kwargs.set_item(&pdef.name, f); },
                                                    "bool" => if let Ok(b) = val.parse::<bool>() { let _ = kwargs.set_item(&pdef.name, b); },
                                                    _ => { let _ = kwargs.set_item(&pdef.name, val); }
                                                }
                                            }
                                        },
                                         "body" => {
                                            if !body_bytes.is_empty() {
                                                if let Ok(json_val) = serde_json::from_slice::<serde_json::Value>(&body_bytes) {
                                                    if pdef.param_type == "json" {
                                                        let _ = kwargs.set_item(&pdef.name, json_to_py(py, &json_val));
                                                    } else if let Some(field_val) = json_val.get(&pdef.name) {
                                                        match pdef.param_type.as_str() {
                                                            "int" => if let Some(i) = field_val.as_i64() { let _ = kwargs.set_item(&pdef.name, i); },
                                                            "float" => if let Some(f) = field_val.as_f64() { let _ = kwargs.set_item(&pdef.name, f); },
                                                            "bool" => if let Some(b) = field_val.as_bool() { let _ = kwargs.set_item(&pdef.name, b); },
                                                            "str" => if let Some(s) = field_val.as_str() { let _ = kwargs.set_item(&pdef.name, s); },
                                                            _ => { let _ = kwargs.set_item(&pdef.name, json_to_py(py, field_val)); }
                                                        }
                                                    }
                                                }
                                            }
                                        },
                                        "header" => {
                                            // Handle headers (FastAPI-style: hyphens converted to underscores)
                                            // Header(alias="...") is handled by name being the alias if provided in Python
                                            let header_name = pdef.name.replace('_', "-");
                                            if let Some(val) = parts.headers.get(&header_name) {
                                                if let Ok(s) = val.to_str() {
                                                    let _ = kwargs.set_item(&pdef.name, s);
                                                }
                                            } else {
                                                // Fallback to exact name if hyphenated version not found
                                                if let Some(val) = parts.headers.get(&pdef.name) {
                                                    if let Ok(s) = val.to_str() {
                                                        let _ = kwargs.set_item(&pdef.name, s);
                                                    }
                                                }
                                            }
                                        },
                                        "cookie" => {
                                            if let Some(cookie_header) = parts.headers.get(axum::http::header::COOKIE) {
                                                if let Ok(cookie_str) = cookie_header.to_str() {
                                                    for cookie in cookie::Cookie::split_parse(cookie_str) {
                                                        if let Ok(c) = cookie {
                                                            if c.name() == pdef.name {
                                                                let _ = kwargs.set_item(&pdef.name, c.value());
                                                                break;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        },
                                        "form" => {
                                            if !body_bytes.is_empty() {
                                                for (k, v) in form_urlencoded::parse(&body_bytes) {
                                                    if k == pdef.name {
                                                        let _ = kwargs.set_item(&pdef.name, v);
                                                        break;
                                                    }
                                                }
                                            }
                                        },
                                        "file" => {
                                            if let Some((filename, ct, data)) = pre_extracted.get(&pdef.name) {
                                                let dapil = py.import("dapil").unwrap();
                                                let upload_file_cls = dapil.getattr("UploadFile").unwrap();
                                                
                                                let io = py.import("io").unwrap();
                                                let bytes_io = io.call_method1("BytesIO", (pyo3::types::PyBytes::new(py, data),)).unwrap();
                                                
                                                let file_kwargs = PyDict::new(py);
                                                if let Some(f) = filename { file_kwargs.set_item("filename", f).unwrap(); }
                                                if let Some(c) = ct { file_kwargs.set_item("content_type", c).unwrap(); }
                                                file_kwargs.set_item("file", bytes_io).unwrap();
                                                
                                                let upload_file = upload_file_cls.call((), Some(&file_kwargs)).unwrap();
                                                let _ = kwargs.set_item(&pdef.name, upload_file);
                                            }
                                        },
                                        _ => {}
                                    }
                                }

                                // Validation logic
                                for pdef in &params_schema {
                                    if let Some(val_obj) = kwargs.get_item(&pdef.name).ok().flatten() {
                                        if let Ok(i) = val_obj.extract::<i64>() {
                                            let f = i as f64;
                                            if let Some(v) = pdef.gt { if f <= v { return ExecResult::Done(axum::response::Response::builder().status(422).body(format!("{} must be greater than {}", pdef.name, v).into()).unwrap()); } }
                                            if let Some(v) = pdef.ge { if f < v { return ExecResult::Done(axum::response::Response::builder().status(422).body(format!("{} must be greater than or equal to {}", pdef.name, v).into()).unwrap()); } }
                                            if let Some(v) = pdef.lt { if f >= v { return ExecResult::Done(axum::response::Response::builder().status(422).body(format!("{} must be less than {}", pdef.name, v).into()).unwrap()); } }
                                            if let Some(v) = pdef.le { if f > v { return ExecResult::Done(axum::response::Response::builder().status(422).body(format!("{} must be less than or equal to {}", pdef.name, v).into()).unwrap()); } }
                                        } else if let Ok(f) = val_obj.extract::<f64>() {
                                            if let Some(v) = pdef.gt { if f <= v { return ExecResult::Done(axum::response::Response::builder().status(422).body(format!("{} must be greater than {}", pdef.name, v).into()).unwrap()); } }
                                            if let Some(v) = pdef.ge { if f < v { return ExecResult::Done(axum::response::Response::builder().status(422).body(format!("{} must be greater than or equal to {}", pdef.name, v).into()).unwrap()); } }
                                            if let Some(v) = pdef.lt { if f >= v { return ExecResult::Done(axum::response::Response::builder().status(422).body(format!("{} must be less than {}", pdef.name, v).into()).unwrap()); } }
                                            if let Some(v) = pdef.le { if f > v { return ExecResult::Done(axum::response::Response::builder().status(422).body(format!("{} must be less than or equal to {}", pdef.name, v).into()).unwrap()); } }
                                        } else if let Ok(s) = val_obj.extract::<String>() {
                                            if let Some(v) = pdef.min_length { if s.len() < v { return ExecResult::Done(axum::response::Response::builder().status(422).body(format!("{} length must be at least {}", pdef.name, v).into()).unwrap()); } }
                                            if let Some(v) = pdef.max_length { if s.len() > v { return ExecResult::Done(axum::response::Response::builder().status(422).body(format!("{} length must be at most {}", pdef.name, v).into()).unwrap()); } }
                                            if let Some(p) = &pdef.pattern {
                                                if let Ok(re) = regex::Regex::new(p) {
                                                    if !re.is_match(&s) { return ExecResult::Done(axum::response::Response::builder().status(422).body(format!("{} does not match pattern {}", pdef.name, p).into()).unwrap()); }
                                                }
                                            }
                                        }
                                    }
                                }

                                let mut args_vec = Vec::new();
                                args_vec.push(py_req.into_any());
                                let args = pyo3::types::PyTuple::new(py, args_vec).unwrap();

                                match h.0.bind(py).call(args, Some(&kwargs)) {
                                    Ok(res) => {
                                        if is_async {
                                             let fut = pyo3_async_runtimes::tokio::into_future(res).unwrap();
                                             ExecResult::Future(Box::pin(fut))
                                        } else {
                                             ExecResult::Done(py_response_to_axum(py, Some(&res)))
                                        }
                                    }
                                    Err(e) => {
                                        ExecResult::Done(handle_py_error(py, e))
                                    }
                                }
                            });

                            match result {
                                ExecResult::Done(resp) => resp,
                                ExecResult::Future(fut) => {
                                    match fut.await {
                                        Ok(py_obj) => Python::with_gil(|py| py_response_to_axum(py, Some(py_obj.bind(py)))),
                                        Err(e) => Python::with_gil(|py| handle_py_error(py, e)),
                                    }
                                }
                            }
                        }).await
                    }
                };

                router = match method.as_str() {
                    "GET" => router.route(&axum_path, routing::get(run_handler)),
                    "POST" => router.route(&axum_path, routing::post(run_handler)),
                    "PUT" => router.route(&axum_path, routing::put(run_handler)),
                    "DELETE" => router.route(&axum_path, routing::delete(run_handler)),
                    _ => router.route(&axum_path, routing::get(run_handler)),
                };
            }

            for mw_arc in &middlewares_copy {
                let mw_arc = mw_arc.clone();
                let locals_clone = locals_arc.clone();
                
                let is_mw_async = Python::with_gil(|py| {
                    match mw_arc.bind(py).getattr("dispatch") {
                        Ok(dispatch_method) => match py.import("inspect") {
                            Ok(inspect) => match inspect.call_method1("iscoroutinefunction", (dispatch_method,)) {
                                Ok(res) => res.extract::<bool>().unwrap_or(false),
                                Err(_) => false,
                            },
                            Err(_) => false,
                        },
                        Err(_) => false,
                    }
                });

                router = router.layer(axum::middleware::from_fn(move |req: AxumRequest, next: axum::middleware::Next| {
                    let mw_arc = mw_arc.clone();
                    let locals_clone = locals_clone.clone();
                    async move {
                        let locals_for_scope = Python::with_gil(|py| locals_clone.clone_ref(py));
                        pyo3_async_runtimes::tokio::scope(locals_for_scope, async move {
                            let (call_tx, mut _call_rx) = mpsc::channel(1);
                            
                            let (parts, body) = req.into_parts();
                            let body_bytes = axum::body::to_bytes(body, 10 * 1024 * 1024).await.unwrap_or_default();
                            let body_clone = body_bytes.clone();
                            let parts_clone = parts.clone();

                            enum MiddlewareExecResult {
                                Done(axum::response::Response),
                                CallNext(AxumRequest),
                                Future(std::pin::Pin<Box<dyn std::future::Future<Output = PyResult<PyObject>> + Send>>),
                            }

                            let result = Python::with_gil(|py| -> MiddlewareExecResult {
                                let query_string = parts_clone.uri.query().unwrap_or("").to_string();
                                let req_obj = crate::Request {
                                    method: parts_clone.method.to_string(),
                                    path: parts_clone.uri.path().to_string(),
                                    query_string: query_string.clone(),
                                    headers: parts_clone.headers.clone(),
                                    body_bytes: body_bytes.clone(),
                                };
                                let py_req = Bound::new(py, req_obj).expect("failed to create Request");

                                let call_next_py = Bound::new(py, PyCallNext { tx: call_tx }).unwrap().into_any().unbind();

                                match mw_arc.bind(py).call_method1("dispatch", (py_req, call_next_py)) {
                                    Ok(res) => {
                                        if is_mw_async {
                                            let fut = pyo3_async_runtimes::tokio::into_future(res).unwrap();
                                            MiddlewareExecResult::Future(Box::pin(fut))
                                        } else {
                                            MiddlewareExecResult::Done(py_response_to_axum(py, Some(&res)))
                                        }
                                    }
                                    Err(e) => {
                                        if e.is_instance_of::<pyo3::exceptions::PyStopIteration>(py) {
                                            MiddlewareExecResult::CallNext(AxumRequest::from_parts(parts_clone, Body::from(body_clone)))
                                        } else {
                                            MiddlewareExecResult::Done(handle_py_error(py, e))
                                        }
                                    }
                                }
                            });

                            match result {
                                MiddlewareExecResult::Done(resp) => resp,
                                MiddlewareExecResult::CallNext(original_req) => {
                                    next.run(original_req).await
                                }
                                MiddlewareExecResult::Future(fut) => {
                                    match fut.await {
                                        Ok(py_obj) => Python::with_gil(|py| py_response_to_axum(py, Some(py_obj.bind(py)))),
                                        Err(e) => Python::with_gil(|py| handle_py_error(py, e)),
                                    }
                                }
                            }
                        }).await
                    }
                }));
            }

            if is_routes_empty {
                 router = router.route("/", routing::get(|| async { "Dapil is running!" }));
            }

            let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

            let addr = format!("{}:{}", host, port);
            
            let socket = socket2::Socket::new(
                if addr.contains(':') && addr.split(':').next().unwrap().contains('.') { socket2::Domain::IPV4 } else { socket2::Domain::IPV6 },
                socket2::Type::STREAM,
                None,
            ).expect("Failed to create socket");

            socket.set_reuse_address(true).expect("Failed to set reuse address");
            #[cfg(all(unix, not(target_os = "solaris"), not(target_os = "illumos")))]
            socket.set_reuse_port(true).expect("Failed to set reuse port");
            socket.set_nonblocking(true).expect("Failed to set nonblocking");

            let address: std::net::SocketAddr = addr.parse().expect("Failed to parse address");
            socket.bind(&address.into()).expect("Failed to bind socket");
            socket.listen(1024).expect("Failed to listen");

            let listener = TcpListener::from_std(socket.into()).expect("Failed to convert socket");

            info!("Dapil serving on http://{}", addr);

            {
                let server_task = async move {
                    axum::serve(listener, router)
                        .with_graceful_shutdown(async move {
                            let _ = shutdown_rx.await;
                        })
                        .await
                };

                tokio::pin!(server_task);

                loop {
                    tokio::select! {
                        res = &mut server_task => {
                            if let Err(e) = res {
                                println!("Axum server error: {}", e);
                            }
                            break;
                        }
                        _ = tokio::signal::ctrl_c() => {
                            println!("Shutdown signal received, starting graceful shutdown...");
                            let _ = shutdown_tx.send(());
                            
                            tokio::select! {
                                res = &mut server_task => {
                                    if let Err(e) = res {
                                        println!("Axum server error during shutdown: {}", e);
                                    }
                                    println!("Server stopped gracefully");
                                }
                                _ = tokio::time::sleep(tokio::time::Duration::from_secs(2)) => {
                                    println!("Shutdown timeout exceeded, force stopping network...");
                                }
                                _ = tokio::signal::ctrl_c() => {
                                    println!("Second Ctrl+C detected, force stopping process...");
                                    std::process::exit(130);
                                }
                            }
                            break;
                        }
                    }
                }
            } 

            println!("Axum server fully dropped");
            
            Ok::<(), PyErr>(())
        }).unwrap();
        
        py.check_signals().unwrap();
        println!("Server stopped gracefully");
        Ok(())
    }
}



#[pyfunction]
fn setup_logging() {
    let _ = pyo3_log::try_init();
}

#[pymodule]
fn _dapil(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<App>()?;
    m.add_class::<Request>()?;
    m.add_function(wrap_pyfunction!(setup_logging, m)?)?;
    Ok(())
}

