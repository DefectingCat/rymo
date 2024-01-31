use anyhow::{anyhow, Result};
use bytes::Bytes;
use log::error;
use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    sync::Arc,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

pub mod http;

#[derive(Debug)]
pub struct Rymo<'a, F, Fut>
where
    F: Fn() -> Fut + 'static + Send + Sync,
    Fut: Future<Output = (i32, Bytes)> + Send,
{
    pub port: &'a str,
    pub routes: Arc<RwLock<HashMap<&'static str, F>>>,
}

impl<'a, F, Fut> Rymo<'a, F, Fut>
where
    F: Fn() -> Fut + 'static + Send + Sync,
    Fut: Future<Output = (i32, Bytes)> + Send,
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

    pub async fn get(&mut self, path: &'static str, handler: F) {
        let mut routes = self.routes.write().await;
        routes.entry(path).or_insert(handler);
    }
}

pub async fn process<F, Fut>(
    mut socket: TcpStream,
    routes: Arc<RwLock<HashMap<&'static str, F>>>,
) -> Result<()>
where
    F: Fn() -> Fut + 'static + Send + Sync,
    Fut: Future<Output = (i32, Bytes)> + Send,
{
    let (reader, mut writer) = socket.split();

    let headers = read_headers(reader).await?;
    let mut headers: VecDeque<_> = headers.lines().collect();
    let route = headers.pop_front().ok_or(anyhow!(""))?;
    let headers = collect_headers(headers.into());

    let request_path: Vec<_> = route.split(' ').collect();
    let request_path = request_path.get(1).ok_or(anyhow!(""))?; // TODO error response

    let routes = routes.read().await;
    let route_handler = routes.get(request_path);
    match route_handler {
        Some(handler) => {
            let resp = handler().await;
            let response = format!("HTTP/1.1 {}\r\n\r\n", resp.0);
            let response = [response.as_bytes(), &resp.1].concat();
            dbg!(&request_path, &headers);
            writer.write_all(&response).await?;
            Ok(())
        }
        None => todo!(), // 404
    }

    /* let response = format!("HTTP/1.1 {}\r\n\r\n", Status::Ok);
    let response = format!("{}hello world", response);
    dbg!(&request_path, &headers, &response);
    writer.write_all(response.as_bytes()).await?;
    Ok(()) */
}

/// Read bytes from reader to string
/// but not common headers, include first line like GET / HTTP/1.1
pub async fn read_headers<R>(reader: R) -> Result<String>
where
    R: AsyncRead + std::marker::Unpin,
{
    let mut request_string = String::new();
    let mut reader = BufReader::new(reader);
    loop {
        let byte = reader.read_line(&mut request_string).await?;
        if byte < 3 {
            break;
        }
    }
    Ok(request_string)
}

/// Collect request string with Hashmap to headers.
pub fn collect_headers(request: Vec<&str>) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    request.iter().for_each(|header| {
        if let Some(head) = header.split_once(": ") {
            headers
                .entry(head.0.to_string())
                .or_insert(head.1.to_string());
        }
    });
    headers
}
