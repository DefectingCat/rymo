use crate::{
    error::{Error, Result},
    http::mime::{read_mime, HTML_UTF_8},
    middleware::Middleware,
    request::{drop_body, read_body, read_headers, Request},
    response::{Response, Status},
    route::Route,
    utils::find_directory,
};
use futures::Future;
use log::{error, info};
use std::{
    collections::{BTreeMap, HashMap},
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    fs,
    io::{AsyncRead, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

// pub type Routes<F> = Arc<RwLock<HashMap<&'static str, HashMap<&'static str, F>>>>;
pub type Routes<F, Fut, M> =
    Arc<RwLock<HashMap<&'static str, HashMap<&'static str, Route<F, Fut, M>>>>>;
pub type AssetsRoutes = Arc<RwLock<BTreeMap<&'static str, PathBuf>>>;
pub type Middlewares<M> = Arc<RwLock<Vec<M>>>;

pub struct Rymo<'a, F, Fut, M>
where
    F: Fn(Request, Response) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<(Request, Response)>> + Send,
    M: Middleware + Send + Sync + 'static,
{
    /// Current listen port
    pub port: &'a str,
    /// Registries routes
    ///
    /// ```not_rust
    /// route_path: {
    ///     http_method: route_handler
    /// }
    /// ```
    pub routes: Routes<F, Fut, M>,
    /// Static assets routes
    ///
    /// ```not_rust
    /// {
    /// assets_route_path: assets_file_path
    /// }
    /// ```
    pub assets_routes: AssetsRoutes,
    pub middlewares: Middlewares<M>,
}

impl<'a, F, Fut, M> Rymo<'a, F, Fut, M>
where
    F: Fn(Request, Response) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<(Request, Response)>> + Send + Sync + Middleware + 'static,
    M: Middleware + Send + Sync + 'static,
{
    #[inline]
    pub fn new(port: &'a str) -> Self {
        Self {
            port,
            routes: Arc::new(RwLock::new(HashMap::new())),
            assets_routes: Arc::new(RwLock::new(BTreeMap::new())),
            middlewares: Arc::new(RwLock::new(vec![])),
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
            let assets_routes = self.assets_routes.clone();
            let task = async move {
                let mut socket = socket;
                match process(&mut socket, routes, assets_routes).await {
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
    pub async fn assets(&self, route_path: &'static str, assets_path: &Path, _handler: F) {
        let mut routes = self.assets_routes.write().await;
        routes
            .entry(route_path)
            .or_insert(assets_path.to_path_buf());
    }

    pub async fn middleware(&self, middleware: M) {
        let mut middlewares = self.middlewares.write().await;
        middlewares.push(middleware);
    }
}

#[inline]
pub async fn static_handler(_req: Request, res: Response) -> anyhow::Result<Response> {
    Ok(res)
}

/// Static assets handler
async fn assets_handler(
    req: Request,
    mut res: Response,
    assets_key: Option<&str>,
    assets_path: &Path,
    is_file: bool,
) -> Result<(Request, Response)> {
    let mut path = assets_path.to_path_buf();
    // use request path as parent path
    let parent = &req.path.to_str();
    // find static assets child directory
    let directory = match assets_key {
        Some(key) if parent.is_some() => find_directory(key, parent.unwrap()),
        _ => None,
    };
    if let Some(d) = directory {
        path.push(d);
    }

    let mime = if is_file {
        let ext = req
            .path
            .extension()
            .unwrap_or(OsStr::new(""))
            .to_str()
            .unwrap_or("");
        read_mime(ext)
    } else {
        path.push("index.html");
        HTML_UTF_8
    };
    let file = fs::read(path).await?;
    res.headers
        .insert("Content-Type".to_owned(), mime.to_owned());
    res.body = file.into();
    Ok((req, res))
}

/// Registry route's handler
macro_rules! http_handler {
    ($fn_name:ident) => {
        impl<'a, F, Fut, M> Rymo<'a, F, Fut, M>
        where
            F: Fn(Request, Response) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = anyhow::Result<(Request, Response)>> + Send,
            M: Middleware + Send + Sync + 'static,
        {
            pub async fn $fn_name(&self, path: &'static str, handler: F) {
                let mut routes = self.routes.write().await;
                let path_handler = routes.entry(path).or_default();
                let middlewares = self.middlewares.clone();
                // route
                let route = Route::new(handler, middlewares);
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

/// Process registried handler
///
/// And build req and res.
#[inline]
pub async fn process<F, Fut, M>(
    socket: &mut TcpStream,
    routes: Routes<F, Fut, M>,
    assets_routes: AssetsRoutes,
) -> Result<()>
where
    F: Fn(Request, Response) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<(Request, Response)>> + Send + Sync + Middleware,
    M: Middleware + Send + Sync + 'static,
{
    let socket_ref = socket2::SockRef::from(&socket);
    let (reader, mut writer) = socket.split();

    // build client request
    let (headers, reader) = read_headers(reader)
        .await
        .map_err(|e| Error::BadRequest(format!("read headers failed {}", e)))?;
    let req = Request::parse_from_bytes(headers.clone())
        .map_err(|e| Error::BadRequest(format!("parse headers from bytes failed {}", e)))?;
    // build response
    let mut res = Response::default();

    /* let ka = socket2::TcpKeepalive::new()
        .with_time(Duration::from_secs(10))
        .with_interval(Duration::from_secs(10));
    socket_ref.set_tcp_keepalive(&ka)?;
    // keep alive
    match req.headers.get(&"connection".to_string()) {
        Some(ka) if ka == "keep-alive" => {
            dbg!(ka);
        }
        _ => {}
    } */

    // Registries routes
    let routes = routes.read().await;
    let req_str = req.path.to_string_lossy();
    // the request path is file path or not
    let is_file = req_str.contains('.') && !req_str.ends_with('/');
    // if it's file, use it's parent path
    // GET /index.html use /
    let req_path = if is_file {
        req.path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or(PathBuf::from(&req.path))
    } else {
        req.path.to_path_buf()
    };

    // static assets routes
    let assets_routes = assets_routes.read().await;
    // loop for assets routes
    let req_path_str = req_path.to_string_lossy();
    let (x, mut y) = (0, 1);
    let (key, assets_path) = loop {
        if y > req_path_str.len() {
            break (None, None);
        }
        let key = &req_path_str[x..y];
        let assets_path = assets_routes.get(key);
        match assets_path {
            Some(path) => {
                break (key.into(), Some(path));
            }
            None => {
                y += 1;
            }
        }
    };

    let response = match assets_path {
        // handle static serve
        Some(path) => assets_handler(req, res, key, path, is_file).await?.into(),
        // handle regular routes
        None => {
            let route_handler = routes.get(req_path_str.as_ref());
            handle_route(route_handler, req, res, reader).await?
        }
    };

    // let response = handle_route(route_handler, req, reader).await?;
    writer.write_all(&response).await?;
    writer.flush().await?;
    Ok(())
}

async fn handle_route<F, Fut, R, M>(
    route_handler: Option<&HashMap<&str, Route<F, Fut, M>>>,
    mut req: Request,
    mut res: Response,
    reader: R,
) -> Result<(Request, Response)>
where
    F: Fn(Request, Response) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<(Request, Response)>> + Send,
    M: Middleware + Send + Sync + 'static,
    R: AsyncRead + Unpin,
{
    // parse body
    let content_len = req.headers.get("content-length");
    let (req, res) = match route_handler {
        Some(handler) => {
            let method = handler.get(req.method.to_lowercase().as_str());
            if let Some(len) = content_len {
                let (body, _) = read_body(reader, len)
                    .await
                    .map_err(Error::InternalServerError)?;
                req.body = body;
            }
            match method {
                Some(route_handler) => (route_handler.handler)(req, res).await?,
                None => {
                    res.status = Status::MethodNotAllowed;
                    (req, res)
                } // Method not allow
            }
        }
        None => {
            drop_body(reader, content_len.map(|c| c.as_str())).await?;
            res.status = Status::NotFound;
            (req, res)
        } // 404
    };
    Ok((req, res))
}
