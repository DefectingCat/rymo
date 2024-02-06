#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid request {0}")]
    InvalidRequest(String),
    #[error("server internal error {0}")]
    UnknownError(#[from] anyhow::Error),
}

pub type Result<T, E = Error> = anyhow::Result<T, E>;
