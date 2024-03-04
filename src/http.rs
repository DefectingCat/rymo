use std::{collections::HashMap, fmt::Display};

use anyhow::{bail, Result};
use bytes::{BufMut, Bytes, BytesMut};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader};

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
    pub path: &'static str,
    pub method: &'static str,
    pub headers: HashMap<&'static str, &'static str>,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            path: "",
            method: "",
            headers: HashMap::new(),
        }
    }
}

impl Request {
    pub fn parse_from_bytes(bytes: Bytes) -> Result<Self> {
        let mut req = Self::default();

        let lines = bytes
            .split(|&b| b == b'\n')
            .map(|line| line.strip_suffix(b"\r").unwrap_or(line));
        lines
            .filter(|l| l.len() > 0)
            .enumerate()
            .try_for_each(|(i, l)| {
                // the first line is route path
                // GET /v1/ HTTP/1.1
                if i == 0 {
                    let route = l.split(|&b| b == b' ');
                    route.enumerate().try_for_each(|(i, r)| {
                        let str = std::str::from_utf8(&r)?;
                        match i {
                            0 => {
                                req.method = str;
                                anyhow::Ok(())
                            }
                            1 => anyhow::Ok(()),
                            2 => anyhow::Ok(()),
                            _ => bail!(":"),
                        }
                    })
                } else {
                    let head = std::str::from_utf8(&l)?;
                    dbg!(head);
                    Ok(())
                }
            })?;
        Ok(req)
    }
}

/// Read bytes from reader to string
/// but not common headers, include first line like GET / HTTP/1.1
/// 13 10 13 10
/// \r \n \r \n
pub async fn read_headers<R>(mut reader: R) -> Result<Bytes>
where
    R: AsyncRead + Unpin,
{
    let mut buffer = BytesMut::with_capacity(512);
    loop {
        let n = reader.read_u8().await?;
        buffer.put_u8(n);
        let len = buffer.len();
        if len < 4 {
            // TODO: handle header less than 4 bytes
        } else {
            let last_four = &buffer[len - 4..len];
            if last_four == b"\r\n\r\n" {
                println!("breaking");
                break;
            }
        }
    }
    let headers = buffer.freeze();
    Ok(headers.clone())
}

/// Collect request string with Hashmap to headers.
pub fn collect_headers(request: Bytes) -> HashMap<String, String> {
    todo!();
}
