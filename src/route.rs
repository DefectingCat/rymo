use std::path::PathBuf;

use futures::Future;

use crate::{http::request::Request, Response};

pub enum Method {
    GET,
    HEAD,
    POST,
    PUT,
    DELETE,
    CONNECT,
    OPTIONS,
    TRACE,
    PATCH,
}

impl From<Method> for &str {
    fn from(value: Method) -> Self {
        use Method::*;

        match value {
            GET => "GET",
            HEAD => "HEAD",
            POST => "POST",
            PUT => "PUT",
            DELETE => "DELETE",
            CONNECT => "CONNECT",
            OPTIONS => "OPTIONS",
            TRACE => "TRACE",
            PATCH => "PATCH",
        }
    }
}

pub struct Route<F, Fut>
where
    F: Fn(Request, Response) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<Response>> + Send,
{
    pub path: &'static str,
    // pub method: Method,
    pub handler: Option<F>,
    pub is_assets: bool,
    pub assets_path: Option<PathBuf>,
}

impl<F, Fut> Route<F, Fut>
where
    F: Fn(Request, Response) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<Response>> + Send,
{
    pub fn new(
        path: &'static str,
        handler: Option<F>,
        is_assets: bool,
        assets_path: Option<PathBuf>,
    ) -> Self {
        Self {
            path,
            handler,
            is_assets,
            assets_path,
        }
    }
}
