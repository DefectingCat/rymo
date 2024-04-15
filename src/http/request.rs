use std::{collections::HashMap, io::ErrorKind, path::PathBuf};

use anyhow::{anyhow, bail, Result};
use bytes::{BufMut, Bytes, BytesMut};
use log::trace;
use tokio::io::{self, AsyncRead, AsyncReadExt};

pub struct Request {
    pub path: PathBuf,
    pub method: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            method: "".to_owned(),
            version: "".to_owned(),
            headers: HashMap::new(),
            body: Bytes::new(),
        }
    }
}

impl Request {
    /// Parse request from HTTP header's bytes that read from tcp.
    pub fn parse_from_bytes(bytes: Bytes) -> Result<Self> {
        let mut req = Self::default();

        let collect_headers = |(i, l): (usize, &[u8])| {
            // the first line is route path
            // GET /v1/ HTTP/1.1
            if i == 0 {
                let route = l.split(|&b| b == b' ');
                let (method, path, version) = route.enumerate().try_fold(
                    (String::new(), PathBuf::new(), String::new()),
                    fold_first_line,
                )?;
                req.method = method;
                req.path = path;
                req.version = version;
                anyhow::Ok(())
                // the second line is headers until \r\n\r\n
            } else {
                let heads = std::str::from_utf8(l)?.split(": ");
                let (k, v) = heads
                    .enumerate()
                    .try_fold((String::new(), String::new()), fold_headers)?;
                req.headers.entry(k).or_insert(v);
                Ok(())
            }
        };

        // GET /v1/ HTTP/1.1\r\nUser-Agent: ua\r\n ..
        bytes
            .split(|&b| b == b'\n')
            .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
            .filter(|l| !l.is_empty())
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
            Ok(0) => {
                break;
            }
            Ok(n) => {
                buffer.put_u8(n);
                let len = buffer.len();
                if len < 4 {
                    continue;
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
    mut prev: (String, PathBuf, String),
    (i, r): (usize, &[u8]),
) -> Result<(String, PathBuf, String)> {
    let str = std::str::from_utf8(r)?;
    match i {
        0 => {
            // GET
            prev.0 = str.to_owned();
            anyhow::Ok(prev)
        }
        1 => {
            // /v1/
            prev.1 = PathBuf::from(str);
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
