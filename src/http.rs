use std::{collections::HashMap, fmt::Display};

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};

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
    pub headers: HashMap<String, String>,
}

impl Request {
    pub fn new<S: Display>(path: S, method: S, headers: HashMap<String, String>) -> Self {
        Self {
            path: path.to_string(),
            method: method.to_string(),
            headers,
        }
    }
}

/// Read bytes from reader to string
/// but not common headers, include first line like GET / HTTP/1.1
pub async fn read_headers<R>(reader: R) -> Result<String>
where
    R: AsyncRead + Unpin,
{
    let mut request_string = String::new();
    let mut reader = BufReader::new(reader);
    loop {
        let byte = reader.read_line(&mut request_string).await?;
        if byte < 3 {
            break;
        }
    }
    Ok(request_string)
}

/// Collect request string with Hashmap to headers.
pub fn collect_headers(request: Vec<&str>) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    request.iter().for_each(|header| {
        if let Some(head) = header.split_once(": ") {
            headers
                .entry(head.0.to_string())
                .or_insert(head.1.to_string());
        }
    });
    headers
}
