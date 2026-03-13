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
use tower_http::trace::TraceLayer;

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

        // Clone routes to avoid holding a borrow on self during router construction
        let routes_copy: Vec<(String, String, PyHandler)> = self.routes.clone();

        for (path, method, handler) in routes_copy {
            let handler_clone = handler.clone();
            
            // Define the core handler logic once
            let run_handler = move || {
                let h = handler_clone.clone();
                async move {
                    tokio::task::spawn_blocking(move || {
                        Python::with_gil(|py| {
                            let args = pyo3::types::PyTuple::empty(py);
                            match h.0.call1(py, args) {
                                Ok(res) => res.extract::<String>(py).unwrap_or_else(|_| "Error".to_string()),
                                Err(e) => {
                                    e.print(py);
                                    "Internal Server Error".to_string()
                                }
                            }
                        })
                    }).await.unwrap_or_else(|_| "Runtime Error".to_string())
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

        // let router = router.layer(TraceLayer::new_for_http());

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

