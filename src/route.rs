use std::sync::Arc;

use futures::Future;
use tokio::sync::RwLock;

use crate::{middleware::Middleware, request::Request, response::Response};

pub struct Route<F, Fut, M>
where
    F: Fn(Request, Response) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<(Request, Response)>> + Send,
    M: Middleware + Sync,
{
    pub middlewares: Arc<RwLock<Vec<M>>>,
    pub handler: F,
}

impl<F, Fut, M> Route<F, Fut, M>
where
    F: Fn(Request, Response) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<(Request, Response)>> + Send,
    M: Middleware + Sync,
{
    pub fn new(handler: F, middlewares: Arc<RwLock<Vec<M>>>) -> Self {
        Self {
            handler,
            middlewares,
        }
    }

    pub async fn handle(&self, req: Request, res: Response) -> anyhow::Result<(Request, Response)> {
        let middlewares = self.middlewares.read().await;
        (self.handler)(req, res).await
    }
}
