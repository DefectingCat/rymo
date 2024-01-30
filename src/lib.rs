use anyhow::Result;
use std::{collections::HashMap, future::Future};
use tokio::net::TcpListener;

#[derive(Debug)]
pub struct Rymo<'a, 'b, F, Fut>
where
    F: FnOnce() -> Fut + 'static + Send + Sync,
    Fut: Future<Output = ()>,
{
    pub port: &'a str,
    pub handle: HashMap<&'b str, F>,
}

impl<'a, 'b, F, Fut> Rymo<'a, 'b, F, Fut>
where
    F: FnOnce() -> Fut + 'static + Send + Sync,
    Fut: Future<Output = ()>,
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
            let (_socket, _) = listener.accept().await?;
        }
    }

    pub fn get(&mut self, path: &'b str, handler: F) {
        self.handle.entry(path).or_insert(handler);
    }
}
