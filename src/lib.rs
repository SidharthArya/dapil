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
use tokio::sync::oneshot;

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
            routes: Vec::new(),
        }
    }

    fn set_host(&mut self, host: &str) {
        self.host = String::from(host);
    }

    fn set_port(&mut self, port: u16) {
        self.port = port;
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

    fn serve_default(&mut self, py: Python<'_>) {
        self.serve(py);
    }

    fn serve(&mut self, py: Python<'_>) {
        let runtime = Runtime::new().expect("Failed to create Tokio Runtime");
        let mut router = Router::new();

        // High-performance channel for the "Single Actor" GIL model
        let (tx, rx) = flume::unbounded::<(PyHandler, oneshot::Sender<String>)>();

        // Start the dedicated Python Worker thread
        thread::spawn(move || {
            while let Ok((handler, response_tx)) = rx.recv() {
                Python::with_gil(|py| {
                    let args = pyo3::types::PyTuple::empty(py);
                    let result = match handler.0.call1(py, args) {
                        Ok(res) => res.extract::<String>(py).unwrap_or_else(|_| "Error".to_string()),
                        Err(e) => {
                            e.print(py);
                            "Internal Server Error".to_string()
                        }
                    };
                    let _ = response_tx.send(result);
                });
            }
        });

        // Clone routes to avoid holding a borrow on self during router construction
        let routes_copy: Vec<(String, String, PyHandler)> = self.routes.clone();

        for (path, method, handler) in routes_copy {
            let tx_clone = tx.clone();
            
            // Define the dispatcher logic
            let run_handler = move || {
                let h = handler.clone();
                let tx_inner = tx_clone.clone();
                async move {
                    let (resp_tx, resp_rx) = oneshot::channel();
                    let _ = tx_inner.send((h, resp_tx));
                    resp_rx.await.unwrap_or_else(|_| "Runtime Error".to_string())
                }
            };

            router = match method.as_str() {
                "GET" => router.route(&path, routing::get(run_handler)),
                "POST" => router.route(&path, routing::post(run_handler)),
                "PUT" => router.route(&path, routing::put(run_handler)),
                "DELETE" => router.route(&path, routing::delete(run_handler)),
                _ => router.route(&path, routing::get(run_handler)),
            };
        }

        if self.routes.is_empty() {
             router = router.route("/", routing::get(|| async { "Dapil is running!" }));
        }

        py.allow_threads(|| {
            runtime.block_on(async {
                let addr = format!("{}:{}", self.host, self.port);
                let listener = TcpListener::bind(&addr).await.expect("Failed to bind");
                info!("Dapil serving on http://{}", addr);
                axum::serve(listener, router)
                    .with_graceful_shutdown(async {
                        tokio::signal::ctrl_c()
                            .await
                            .expect("failed to install CTRL+C handler");
                        info!("Shutdown signal received, stopping server...");
                    })
                    .await
                    .unwrap();
            });
        });
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

