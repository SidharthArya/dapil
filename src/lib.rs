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
    body::Body,
};
use tokio_stream::StreamExt;

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

#[derive(Debug)]
struct PyHandler(PyObject);

impl Clone for PyHandler {
    fn clone(&self) -> Self {
        Python::with_gil(|py| PyHandler(self.0.clone_ref(py)))
    }
}

unsafe impl Send for PyHandler {}
unsafe impl Sync for PyHandler {}

#[pyclass]
struct App {
    host: String,
    port: u16,
    workers: usize,
    // Store routes as (path, method) -> handler
    routes: Vec<(String, String, PyHandler)>,
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
        }
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

    fn route(&mut self, method: String, path: String, handler: PyObject) {
        self.routes.push((path, method.to_uppercase(), PyHandler(handler)));
    }

    fn get(&mut self, path: String, handler: PyObject) {
        self.route("GET".to_string(), path, handler);
    }

    fn post(&mut self, path: String, handler: PyObject) {
        self.route("POST".to_string(), path, handler);
    }

    fn put(&mut self, path: String, handler: PyObject) {
        self.route("PUT".to_string(), path, handler);
    }

    fn delete(&mut self, path: String, handler: PyObject) {
        self.route("DELETE".to_string(), path, handler);
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
        let runtime = Runtime::new().expect("Failed to create Tokio Runtime");
        self.setup_and_run(py, runtime)
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
                    // Child process: initialize its own runtime and serve
                    let runtime = Runtime::new().expect("Failed to create Tokio Runtime");
                    let _ = self.serve_worker(runtime);
                    std::process::exit(0);
                }
                Err(_) => panic!("Fork failed"),
            }
        }

        // Master process: wait for all children
        for _ in 0..self.workers {
            let _ = wait();
        }
        Ok(())
    }

    fn serve_worker(&mut self, runtime: Runtime) -> PyResult<()> {
        Python::with_gil(|py| {
            self.setup_and_run(py, runtime)
        })
    }

    fn setup_and_run(&mut self, py: Python<'_>, runtime: Runtime) -> PyResult<()> {
        let mut router = Router::new();
        // High-performance channel for the "Single Actor" GIL model
        let (tx, rx) = flume::unbounded::<(PyHandler, std::collections::HashMap<String, String>, oneshot::Sender<ResponseData>)>();

        // Start the dedicated Python Worker thread
        let worker_handle = thread::spawn(move || {
            while let Ok((handler, params, response_tx)) = rx.recv() {
                Python::with_gil(|py| {
                    let args = pyo3::types::PyTuple::empty(py);
                    let kwargs = pyo3::types::PyDict::new(py);
                    for (k, v) in params {
                        // Attempt to convert to int if possible, common FastAPI behavior
                        if let Ok(i) = v.parse::<i64>() {
                            let _ = kwargs.set_item(k, i);
                        } else {
                            let _ = kwargs.set_item(k, v);
                        }
                    }

                    let result_obj = match handler.0.bind(py).call(args, Some(&kwargs)) {
                        Ok(res) => res,
                        Err(e) => {
                            // Check if it's an HTTPException
                            if e.is_instance_of::<pyo3::exceptions::PyException>(py) {
                                // Try to extract status_code and detail
                                let val = e.value(py);
                                let status_code = val.getattr("status_code").and_then(|s| s.extract::<u16>()).unwrap_or(500);
                                let detail = val.getattr("detail").and_then(|d| d.extract::<String>()).unwrap_or_else(|_| "Internal Server Error".to_string());
                                let _ = response_tx.send(ResponseData {
                                    status: status_code,
                                    body: BodyData::Bytes(detail.into_bytes()),
                                    headers: vec![("content-type".to_string(), "text/plain".to_string())],
                                });
                                return;
                            }
                            e.print(py);
                            let _ = response_tx.send(ResponseData {
                                status: 500,
                                body: BodyData::Bytes("Internal Server Error".as_bytes().to_vec()),
                                headers: vec![],
                            });
                            return;
                        }
                    };

                    // Handle different return types
                    if let Ok(s) = result_obj.extract::<String>() {
                        let _ = response_tx.send(ResponseData {
                            status: 200,
                            body: BodyData::Bytes(s.into_bytes()),
                            headers: vec![("content-type".to_string(), "text/plain".to_string())],
                        });
                        return;
                    } 
                    
                    if let Ok(b) = result_obj.extract::<Vec<u8>>() {
                         let _ = response_tx.send(ResponseData {
                            status: 200,
                            body: BodyData::Bytes(b),
                            headers: vec![("content-type".to_string(), "application/octet-stream".to_string())],
                        });
                        return;
                    }

                    // Check if it's a Response or StreamingResponse object
                    let status = result_obj.getattr("status_code").and_then(|s| s.extract::<u16>()).unwrap_or(200);
                    let headers_dict = result_obj.getattr("headers").and_then(|h| h.extract::<std::collections::HashMap<String, String>>()).unwrap_or_default();
                    let mut headers = Vec::new();
                    for (k, v) in headers_dict {
                        headers.push((k, v));
                    }

                    // Determine if it's a stream
                    if result_obj.getattr("content").is_ok() {
                        let content = result_obj.getattr("content").unwrap();
                        
                        // Check for StreamingResponse marker
                        let is_streaming = !content.is_instance_of::<pyo3::types::PyString>() && 
                                           !content.is_instance_of::<pyo3::types::PyBytes>() && 
                                           content.iter().is_ok();

                        if is_streaming {
                            let (chunk_tx, chunk_rx) = mpsc::channel::<Vec<u8>>(10);
                            
                            let _ = response_tx.send(ResponseData {
                                status,
                                body: BodyData::Stream(chunk_rx),
                                headers: headers.clone(),
                            });

                            // Now iterate and send chunks
                            if let Ok(it) = content.iter() {
                                for chunk_res in it {
                                    match chunk_res {
                                        Ok(chunk_obj) => {
                                            let chunk = if let Ok(s) = chunk_obj.extract::<String>() {
                                                s.into_bytes()
                                            } else if let Ok(b) = chunk_obj.extract::<Vec<u8>>() {
                                                b
                                            } else {
                                                vec![]
                                            };
                                            if let Err(_) = chunk_tx.blocking_send(chunk) {
                                                break;
                                            }
                                        }
                                        Err(e) => {
                                            e.print(py);
                                            break;
                                        }
                                    }
                                }
                            }
                            return;
                        } else {
                            // Default bytes/string body
                            let body = if let Ok(s) = content.extract::<String>() {
                                BodyData::Bytes(s.into_bytes())
                            } else if let Ok(b) = content.extract::<Vec<u8>>() {
                                BodyData::Bytes(b)
                            } else {
                                BodyData::Bytes(vec![])
                            };
                            let _ = response_tx.send(ResponseData {
                                status,
                                body,
                                headers,
                            });
                            return;
                        }
                    }

                    // Final fallback
                    let _ = response_tx.send(ResponseData {
                        status: 500,
                        body: BodyData::Bytes("Unsupported result type".into()),
                        headers: vec![],
                    });
                });
            }
        });

        // Clone routes to avoid holding a borrow on self during router construction
        let routes_copy: Vec<(String, String, PyHandler)> = self.routes.clone();

        for (path, method, handler) in routes_copy {
            let tx_clone = tx.clone();
            let axum_path = path.replace('{', ":").replace('}', "");
            
            // Define the dispatcher logic
            let run_handler = move |params: Option<axum::extract::Path<std::collections::HashMap<String, String>>>| {
                let params = params.map(|axum::extract::Path(p)| p).unwrap_or_default();
                let h = handler.clone();
                let tx_inner = tx_clone.clone();
                async move {
                    let (resp_tx, resp_rx) = oneshot::channel();
                    let _ = tx_inner.send((h, params, resp_tx));
                    resp_rx.await.unwrap_or(ResponseData {
                        status: 500,
                        body: BodyData::Bytes("Runtime Error".as_bytes().to_vec()),
                        headers: vec![],
                    })
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

        if self.routes.is_empty() {
             router = router.route("/", routing::get(|| async { "Dapil is running!" }));
        }

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        py.allow_threads(|| {
            runtime.block_on(async {
                let addr = format!("{}:{}", self.host, self.port);
                
                // Use socket2 for SO_REUSEPORT
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

                // We must ensure the router (and its clones of tx) are dropped before we join the worker thread
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
                                
                                // Wait for server to stop with timeout or second Ctrl+C
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
                } // server_task goes out of scope here, dropping everything

                println!("Axum server fully dropped");
                drop(tx);
                
                println!("Joining Python worker thread...");
            });
            let _ = worker_handle.join();
            println!("Python worker thread joined");
        });

        // After releasing the GIL, check if a signal (like Ctrl+C) occurred
        py.check_signals()?;

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
    m.add_function(wrap_pyfunction!(setup_logging, m)?)?;
    Ok(())
}

