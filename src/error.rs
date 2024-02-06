use anyhow::anyhow;

pub enum Error {
    InvalidRequest(String),
    UnknownError(anyhow::Error),
}

impl From<Error> for anyhow::Error {
    fn from(value: Error) -> Self {
        use Error::*;

        match value {
            InvalidRequest(err) => {
                anyhow!("invalid request {}", err)
            }
            UnknownError(err) => err,
        }
    }
}

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Self::UnknownError(value)
    }
}

pub type Result<T, E = Error> = anyhow::Result<T, E>;
