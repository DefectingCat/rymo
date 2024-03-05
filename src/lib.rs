use std::{collections::HashMap, sync::Arc};

use futures::Future;
use log::error;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

pub use http::Response;
use http::{drop_body, read_body, read_headers, IntoResponse, Request, Status};

use crate::error::Result;

pub mod error;
pub mod http;

pub struct Rymo<'a, F, Fut>
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response>,
{
    /// Current listen port
    pub port: &'a str,
    /// Registries routes
    ///
    /// ```not_rust
    /// route_path : {
    ///     http_method: route_handler
    /// }
    /// ```
    pub routes: Arc<RwLock<HashMap<&'static str, HashMap<&'static str, F>>>>,
}

impl<'a, F, Fut> Rymo<'a, F, Fut>
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send,
{
    pub fn new(port: &'a str) -> Self {
        Self {
            port,
            routes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn serve(&self) -> Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;

        loop {
            let (socket, _) = listener.accept().await?;
            let routes = self.routes.clone();
            let task = async move {
                match process(socket, routes).await {
                    Ok(_) => {}
                    Err(err) => {
                        error!("ERROR: handle route failed {}", err);
                    }
                }
            };
            tokio::spawn(task);
        }
    }
}

macro_rules! http_handler {
    ($fn_name:ident) => {
        impl<'a, F, Fut> Rymo<'a, F, Fut>
        where
            F: Fn(Request) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Response>,
        {
            pub async fn $fn_name(&self, path: &'static str, handler: F) {
                let mut routes = self.routes.write().await;
                let path_handler = routes.entry(path).or_default();
                path_handler.entry(stringify!($fn_name)).or_insert(handler);
            }
        }
    };
}
http_handler!(get);
http_handler!(head);
http_handler!(post);
http_handler!(put);
http_handler!(delete);
http_handler!(connect);
http_handler!(options);
http_handler!(trace);
http_handler!(patch);

pub async fn process<F, Fut>(
    mut socket: TcpStream,
    routes: Arc<RwLock<HashMap<&'static str, HashMap<&'static str, F>>>>,
) -> Result<()>
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response>,
{
    let (reader, mut writer) = socket.split();

    // build client request
    let (headers, reader) = read_headers(reader).await?;
    let mut req = Request::parse_from_bytes(headers.clone())?;

    // parse body
    let content_len = req.headers.get("content-length");

    // Registries routes
    let routes = routes.read().await;
    let route_handler = routes.get(req.path.as_str());
    match route_handler {
        Some(handler) => {
            let method = handler.get(req.method.to_lowercase().as_str());
            if let Some(len) = content_len {
                let (body, _) = read_body(reader, len).await?;
                req.body = body;
            }
            match method {
                Some(route_handler) => {
                    let resp = route_handler(req).await.into_response();
                    writer.write_all(&resp).await?;
                    Ok(())
                }
                None => {
                    let response = format!("HTTP/1.1 {}\r\n\r\n", Status::MethodNotAllowed);
                    writer.write_all(response.as_bytes()).await?;
                    Ok(())
                } // Method not allow
            }
        }
        None => {
            drop_body(reader, content_len.map(|c| c.as_str())).await?;
            let response = format!("HTTP/1.1 {}\r\n\r\n", Status::NotFound);
            writer.write_all(response.as_bytes()).await?;
            Ok(())
        } // 404
    }
}
