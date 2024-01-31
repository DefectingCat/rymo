use crate::http::Status;
use anyhow::{anyhow, Result};
use log::error;
use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    sync::{Arc, RwLock},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

pub mod http;

#[derive(Debug)]
pub struct Rymo<'a, 'b, F, Fut>
where
    F: FnOnce() -> Fut + 'static + Send + Sync,
    Fut: Future<Output = (i32, &'b [u8])>,
{
    pub port: &'a str,
    pub handle: Arc<RwLock<HashMap<&'b str, F>>>,
}

impl<'a, 'b, F, Fut> Rymo<'a, 'b, F, Fut>
where
    F: FnOnce() -> Fut + 'static + Send + Sync,
    Fut: Future<Output = (i32, &'b [u8])>,
{
    pub fn new(port: &'a str) -> Self {
        Self {
            port,
            handle: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn serve(&self) -> Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;

        loop {
            let (socket, _) = listener.accept().await?;
            let task = async move {
                match process(socket).await {
                    Ok(_) => {}
                    Err(err) => {
                        error!("ERROR: handle route failed {}", err);
                    }
                }
            };
            tokio::spawn(task);
        }
    }

    pub fn get(&mut self, path: &'b str, handler: F) -> Result<()> {
        let mut routes = self
            .handle
            .write()
            .map_err(|err| anyhow!("lock rwlock failed {}", err))?;
        routes.entry(path).or_insert(handler);
        Ok(())
    }
}

pub async fn process(mut socket: TcpStream) -> Result<()> {
    let (reader, mut writer) = socket.split();

    let headers = read_headers(reader).await?;
    let mut headers: VecDeque<_> = headers.lines().collect();
    let route = headers.pop_front().ok_or(anyhow!(""));
    let headers = collect_headers(headers.into());

    let response = format!("HTTP/1.1 {}\r\n\r\n", Status::Ok);
    let response = format!("{}hello world", response);
    dbg!(&route, &headers, &response);
    writer.write_all(response.as_bytes()).await?;
    Ok(())
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
