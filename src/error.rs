use anyhow::anyhow;

use crate::response::{IntoResponse, Status};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid request {0}")]
    BadRequest(String),
    #[error("server internal error {0}")]
    InternalServerError(anyhow::Error),
}

impl IntoResponse for Error {
    #[inline]
    fn into_response(self) -> Vec<u8> {
        format!("HTTP/1.1 {}\r\n\r\n", Status::InternalServer).into_bytes()
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::InternalServerError(anyhow!("{value}"))
    }
}

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Self::InternalServerError(value)
    }
}

pub type Result<T, E = Error> = anyhow::Result<T, E>;
