use std::{collections::HashMap, path::Path, sync::Arc};

use anyhow::anyhow;
use error::Error;
use futures::Future;
use log::{error, info};
use tokio::{
    fs,
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

use http::request::{drop_body, read_body, read_headers, Request};
pub use http::response::{IntoResponse, Response, Status};

use crate::error::Result;

pub mod error;
pub mod http;
pub mod route;

type Routes<F, Fut> =
    Arc<RwLock<HashMap<&'static str, HashMap<&'static str, route::Route<F, Fut>>>>>;

pub struct Rymo<'a, F, Fut>
where
    F: Fn(Request, Response) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<Response>> + Send,
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
    pub routes: Routes<F, Fut>,
}

impl<'a, F, Fut> Rymo<'a, F, Fut>
where
    F: Fn(Request, Response) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<Response>> + Send + 'static,
{
    #[inline]
    pub fn new(port: &'a str) -> Self {
        Self {
            port,
            routes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start server
    #[inline]
    pub async fn serve(&self) -> Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;

        loop {
            let (socket, addr) = listener.accept().await?;
            info!("accept connection from {}", addr);
            let routes = self.routes.clone();
            let task = async move {
                let mut socket = socket;
                match process(&mut socket, routes).await {
                    Ok(_) => {}
                    Err(err) => {
                        let response = match &err {
                            Error::BadRequest(_) => {
                                format!("HTTP/1.1 {}\r\n\r\n", Status::BadRequest)
                            }
                            Error::InternalServerError(_) => {
                                format!("HTTP/1.1 {}\r\n\r\n", Status::InternalServer)
                            }
                        };
                        let _ = socket.write_all(response.as_bytes()).await;
                        let _ = socket.flush().await;
                        error!("handle route failed {}", err);
                    }
                }
            };
            tokio::spawn(task);
        }
    }

    /// Read target directory and try to find `index.html`
    ///
    /// ## Arguments
    ///
    /// - `route_path`: registry route's path
    /// - `assets_path`: the static assets path
    #[inline]
    pub async fn assets(&self, route_path: &'static str, assets_path: &Path) {
        let mut routes = self.routes.write().await;
        let path_handler = routes.entry(route_path).or_default();
        // route
        let route = route::Route::new(route_path, None, true, Some(assets_path.to_path_buf()));
        path_handler.entry("get").or_insert(route);
    }
}

/// Static assets handler
///
/// TODO: handle deferent file types
#[inline]
async fn assets_handler(
    req: Request,
    mut res: Response,
    assets_path: &Path,
    is_file: bool,
) -> Result<Response> {
    let mut path = assets_path.to_path_buf();
    let index = if is_file {
        path.push(
            req.path
                .file_name()
                .ok_or(anyhow!("read filename from path failed"))?,
        );
        fs::read(path).await?
    } else {
        path.push("index.html");
        fs::read(path).await?
    };
    res.headers.insert(
        "Content-Type".to_owned(),
        "text/html; charset=utf-8".to_owned(),
    );
    res.body = index.into();
    Ok(res)
}

/// Registry route's handler
macro_rules! http_handler {
    ($fn_name:ident) => {
        impl<'a, F, Fut> Rymo<'a, F, Fut>
        where
            F: Fn(Request, Response) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = anyhow::Result<Response>> + Send,
        {
            pub async fn $fn_name(&self, path: &'static str, handler: F) {
                let mut routes = self.routes.write().await;
                let path_handler = routes.entry(path).or_default();
                // route
                let route = route::Route::new(path, Some(handler), false, None);
                path_handler.entry(stringify!($fn_name)).or_insert(route);
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

#[inline]
pub async fn process<F, Fut>(socket: &mut TcpStream, routes: Routes<F, Fut>) -> Result<()>
where
    F: Fn(Request, Response) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<Response>> + Send,
{
    let (reader, mut writer) = socket.split();

    // build client request
    let (headers, reader) = read_headers(reader)
        .await
        .map_err(|e| Error::BadRequest(format!("read headers failed {}", e)))?;
    let mut req = Request::parse_from_bytes(headers.clone())
        .map_err(|e| Error::BadRequest(format!("parse headers from bytes failed {}", e)))?;

    // parse body
    let content_len = req.headers.get("content-length");

    // Registries routes
    let routes = routes.read().await;
    let req_str = req.path.to_string_lossy();
    let is_file = req_str.contains('.') && !req_str.ends_with('/');
    let req_path = if is_file {
        req.path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or(String::from("/"))
    } else {
        req.path.to_string_lossy().to_string()
    };
    let route_handler = routes.get(req_path.as_str());
    // regular routes
    let response = match route_handler {
        Some(handler) => {
            let method = handler.get(req.method.to_lowercase().as_str());
            if let Some(len) = content_len {
                let (body, _) = read_body(reader, len)
                    .await
                    .map_err(Error::InternalServerError)?;
                req.body = body;
            }
            let res = Response::default();
            match method {
                // static serve
                Some(route_handler) if route_handler.is_assets => {
                    let assets_path = route_handler
                        .assets_path
                        .as_ref()
                        .ok_or(anyhow!("cannot find assets path"))?;
                    assets_handler(req, res, assets_path, is_file).await?.into()
                }
                // regular route
                Some(route_handler) => {
                    let handler = &route_handler
                        .handler
                        .as_ref()
                        .ok_or(anyhow!("cannot find route handler"))?;
                    handler(req, res).await?.into()
                }
                None => {
                    let res = format!("HTTP/1.1 {}\r\n\r\n", Status::MethodNotAllowed);
                    res.into_bytes()
                } // Method not allow
            }
        }
        None => {
            drop_body(reader, content_len.map(|c| c.as_str())).await?;
            format!("HTTP/1.1 {}\r\n\r\n", Status::NotFound).into_bytes()
        } // 404
    };
    writer.write_all(&response).await?;
    writer.flush().await?;
    Ok(())
}
