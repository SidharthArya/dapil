mod methods;

use pyo3::prelude::*;
use axum::{
    routing,
    Router,
};
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
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
            
            // Helper to create a service for a given handler
            let make_service = |h: PyHandler| {
                routing::get(move || {
                    let h_inner = h.clone();
                    async move {
                        Python::with_gil(|py| {
                            let res = h_inner.0.call0(py).expect("Failed to call handler");
                            res.extract::<String>(py).expect("Handler must return a string")
                        })
                    }
                })
            };

            router = match method.as_str() {
                "GET" => router.route(&path, make_service(handler_clone)),
                "POST" => {
                    let h_inner = handler_clone.clone();
                    router.route(&path, routing::post(move || {
                        let h_final = h_inner.clone();
                        async move {
                            Python::with_gil(|py| {
                                let res = h_final.0.call0(py).expect("Failed to call handler");
                                res.extract::<String>(py).expect("Handler must return a string")
                            })
                        }
                    }))
                },
                _ => router.route(&path, make_service(handler_clone)),
            };
        }

        if self.routes.is_empty() {
             router = router.route("/", routing::get(|| async { "Dapil is running!" }));
        }

        let router = router.layer(TraceLayer::new_for_http());

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
fn setup_logging(level: Option<String>) {
    let filter = match level {
        Some(l) => EnvFilter::builder()
            .with_default_directive(l.parse().unwrap_or(LevelFilter::INFO.into()))
            .from_env_lossy(),
        None => EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into()),
    };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();
}

#[pymodule]
fn _dapil(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<App>()?;
    m.add_function(wrap_pyfunction!(setup_logging, m)?)?;
    Ok(())
}

