use std::{collections::HashMap, fmt::Display, io::ErrorKind};

use anyhow::{anyhow, bail, Result};
use bytes::{BufMut, Bytes, BytesMut};
use log::trace;
use tokio::io::{self, AsyncRead, AsyncReadExt};

pub struct Response(pub Status, pub Bytes);

pub trait IntoResponse {
    fn into_response(self) -> Vec<u8>;
}

impl IntoResponse for Response {
    fn into_response(self) -> Vec<u8> {
        let response = format!("HTTP/1.1 {}\r\n\r\n", self.0);
        [response.as_bytes(), &self.1].concat()
    }
}

#[derive(Debug)]
pub enum Status {
    Ok,
    InternalServer,
    NotFound,
    MethodNotAllowed,
}

impl From<&Status> for &str {
    fn from(val: &Status) -> Self {
        use Status::*;

        match val {
            Ok => "200 OK",
            InternalServer => "500 Internal Server Error",
            NotFound => "404 Not Found",
            MethodNotAllowed => "405 Method Not Allowed",
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status: &'static str = self.into();
        write!(f, "{}", status)
    }
}

pub struct Request {
    pub path: String,
    pub method: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            path: "".to_owned(),
            method: "".to_owned(),
            version: "".to_owned(),
            headers: HashMap::new(),
            body: Bytes::new(),
        }
    }
}

impl Request {
    pub fn parse_from_bytes(bytes: Bytes) -> Result<Self> {
        let mut req = Self::default();

        // GET /v1/ HTTP/1.1\r\nUser-Agent: ua\r\n ..
        let lines = bytes
            .split(|&b| b == b'\n')
            .map(|line| line.strip_suffix(b"\r").unwrap_or(line));

        let collect_headers = |(i, l): (usize, &[u8])| {
            // the first line is route path
            // GET /v1/ HTTP/1.1
            if i == 0 {
                let route = l.split(|&b| b == b' ');
                let (method, path, version) = route.enumerate().try_fold(
                    (String::new(), String::new(), String::new()),
                    fold_first_line,
                )?;
                req.method = method;
                req.path = path;
                req.version = version;
                anyhow::Ok(())
                // the second line is headers until \r\n\r\n
            } else {
                let heads = std::str::from_utf8(&l)?.split(": ");
                let (k, v) = heads
                    .enumerate()
                    .try_fold((String::new(), String::new()), fold_headers)?;
                req.headers.entry(k).or_insert(v);
                Ok(())
            }
        };
        lines
            .filter(|l| l.len() > 0)
            .enumerate()
            .try_for_each(collect_headers)?;

        Ok(req)
    }
}

/// Read bytes from reader to string
/// but not common headers, include first line like GET / HTTP/1.1
/// 13 10 13 10
/// \r \n \r \n
pub async fn read_headers<R>(mut reader: R) -> Result<(Bytes, R)>
where
    R: AsyncRead + Unpin,
{
    let mut buffer = BytesMut::with_capacity(512);
    loop {
        let res = reader.read_u8().await;
        match res {
            Ok(n) if n == 0 => {
                break;
            }
            Ok(n) => {
                buffer.put_u8(n);
                let len = buffer.len();
                if len < 4 {
                    // TODO: handle header less than 4 bytes
                } else {
                    let last_four = &buffer[len - 4..len];
                    if last_four == b"\r\n\r\n" {
                        trace!("breaking read headers");
                        break;
                    }
                }
            }
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                return Ok((buffer.freeze(), reader));
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }
    let headers = buffer.freeze();
    Ok((headers.clone(), reader))
}

/// Read client request body by it's content-length
pub async fn read_body<R>(mut reader: R, len: &str) -> Result<(Bytes, R)>
where
    R: AsyncRead + Unpin,
{
    let len: usize = len.parse().map_err(|e| anyhow!("{e}"))?;
    let mut buffer = vec![0u8; len];
    reader.read_exact(&mut buffer).await?;
    let buffer = Bytes::copy_from_slice(&buffer);
    Ok((buffer, reader))
}

/// Pull all body into tokio::io::empty
pub async fn drop_body<R>(reader: R, len: Option<&str>) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    if let Some(len) = len {
        let len: u64 = len.parse().map_err(|e| anyhow!("{e}"))?;
        let mut r = reader.take(len);
        let mut null = io::empty();
        io::copy(&mut r, &mut null).await?;
    } else {
    }
    Ok(())
}

// TODO: error handle

/// Collect http first line in headers
/// The first line is route path
/// GET /v1/ HTTP/1.1
///
/// ## Arguments
///
/// - prev: (method, path, version)
fn fold_first_line(
    mut prev: (String, String, String),
    (i, r): (usize, &[u8]),
) -> Result<(String, String, String)> {
    let str = std::str::from_utf8(r)?;
    match i {
        0 => {
            // GET
            prev.0 = str.to_owned();
            anyhow::Ok(prev)
        }
        1 => {
            // /v1/
            prev.1 = str.to_owned();
            anyhow::Ok(prev)
        }
        2 => {
            // HTTP/1.1
            prev.2 = str.to_owned();
            anyhow::Ok(prev)
        }
        _ => bail!(""),
    }
}

/// Collect headers into hashmap
///
/// ## Arguments
///
/// - prev: (key, value) for hashmap
fn fold_headers(mut prev: (String, String), (i, h): (usize, &str)) -> Result<(String, String)> {
    match i {
        // User-Agent: ua
        0 => {
            // User-Agent
            prev.0 = h.to_lowercase().to_owned();
            anyhow::Ok(prev)
        }
        1 => {
            // ua
            prev.1 = h.to_owned();
            anyhow::Ok(prev)
        }
        _ => bail!(""),
    }
}
