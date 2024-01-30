use anyhow::Result;
use bytes::Bytes;
use std::{collections::HashMap, future::Future, pin::Pin};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader},
    net::{TcpListener, TcpStream},
};

#[derive(Debug)]
pub struct Rymo<'a, 'b, F, Fut>
where
    F: FnOnce() -> Fut + 'static + Send + Sync,
    Fut: Future<Output = (i32, Bytes)>,
{
    pub port: &'a str,
    pub handle: HashMap<&'b str, F>,
}

impl<'a, 'b, F, Fut> Rymo<'a, 'b, F, Fut>
where
    F: FnOnce() -> Fut + 'static + Send + Sync,
    Fut: Future<Output = (i32, Bytes)>,
{
    pub fn new(port: &'a str) -> Self {
        Self {
            port,
            handle: HashMap::new(),
        }
    }

    pub async fn serve(&self) -> Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;

        loop {
            let (socket, _) = listener.accept().await?;
            self.process(socket).await?;
        }
    }

    pub fn get(&mut self, path: &'b str, handler: F) {
        self.handle.entry(path).or_insert(handler);
    }

    pub async fn process(&self, mut socket: TcpStream) -> Result<()> {
        let (reader, _writer) = socket.split();

        let headers = read_headers(reader).await?;
        dbg!(&headers);
        Ok(())
    }
}

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
