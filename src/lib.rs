mod methods;

use std::collections::HashMap;
use pyo3::prelude::*;
use axum::{
    routing,
    Router,
};
use std::net::SocketAddr;
use std::string::ToString;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tokio;
use pyo3::types::PyFunction;
use log::{info, warn};

#[pyclass]
struct App {
    host: String,
    port: u16,
    routes: HashMap<String, HashMap<String, extern fn(args: String)>>,
}

#[pymethods]
impl App {
    #[new]
    fn new() -> Self {
        return App {host: "0.0.0.0".to_string(), port: 8080, routes: HashMap::new()};
    }

    fn set_host(&mut self, host: &str){
        self.host = String::from(host);
    }

    fn set_port(&mut self, port: u16){
        self.port = port;
    }

    // fn add_routes(&mut self, router: Router) -> Router{
    // }


    fn serve(&mut self) {
        let runtime = Runtime::new().expect("Failed to create Tokio Runtime");
        let mut app = Router::new();
        app = app.route("/", routing::get(|| async {"Hello World"}));
        runtime.block_on(async {
            let listener = tokio::net::TcpListener::bind((self.host).to_owned() + &*":".to_owned() + &*self.port.to_string()).await.unwrap();
            axum::serve(listener, app).await.unwrap();
        });

    }
}




#[pymodule]
fn dapil(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<App>()?;
    Ok(())
}

