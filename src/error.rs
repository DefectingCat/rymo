use anyhow::anyhow;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid request {0}")]
    InvalidRequest(String),
    #[error("server internal error {0}")]
    InternalServerError(anyhow::Error),
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
