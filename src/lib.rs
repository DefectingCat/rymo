use anyhow::{anyhow, Result};
use bytes::Bytes;
use http::{collect_headers, read_headers, Request, Status};
use log::error;
use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    pin::Pin,
    sync::Arc,
};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

pub mod http;

pub type Response = Pin<Box<dyn Future<Output = (Status, Bytes)> + Send>>;
type Job = fn(Request) -> Response;
type Routes = Arc<RwLock<HashMap<&'static str, HashMap<&'static str, Job>>>>;

pub struct Rymo<'a> {
    /// Current listen port
    pub port: &'a str,
    /// Registried routes
    ///
    /// ```not_rust
    /// route_path : {
    ///     http_method: route_handler
    /// }
    /// ```
    pub routes: Routes,
}

impl<'a> Rymo<'a> {
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
        impl<'a> Rymo<'a> {
            pub async fn $fn_name(&self, path: &'static str, handler: Job) {
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

pub async fn process(mut socket: TcpStream, routes: Routes) -> Result<()> {
    let (reader, mut writer) = socket.split();

    let headers = read_headers(reader).await?;
    let mut headers: VecDeque<_> = headers.lines().collect();
    let route = headers.pop_front().ok_or(anyhow!(""))?;
    let headers = collect_headers(headers.into());

    let request_path: Vec<_> = route.split(' ').collect();
    let request_method = request_path.first().ok_or(anyhow!(""))?; // read method failed
    let request_path = request_path.get(1).ok_or(anyhow!(""))?; // TODO error response

    // build client request
    let req = Request::new(request_path, request_method, headers);

    // Registried routes
    let routes = routes.read().await;
    let route_handler = routes.get(request_path);
    match route_handler {
        Some(handler) => {
            let method = handler.get(request_method.to_lowercase().as_str());
            match method {
                Some(route_handler) => {
                    let resp = route_handler(req).await;
                    let response = format!("HTTP/1.1 {}\r\n\r\n", resp.0);
                    let response = [response.as_bytes(), &resp.1].concat();
                    // dbg!(&request_path, &headers);
                    writer.write_all(&response).await?;
                    Ok(())
                }
                None => todo!(), // Method not allow
            }
        }
        None => todo!(), // 404
    }
}
