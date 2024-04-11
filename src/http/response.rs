use std::{collections::HashMap, fmt::Display};

use bytes::Bytes;

pub struct Response {
    headers: HashMap<String, String>,
    body: Bytes,
    status: Status,
}

pub trait IntoResponse {
    fn into_response(self) -> Vec<u8>;
}

impl IntoResponse for Response {
    fn into_response(self) -> Vec<u8> {
        let headers = self
            .headers
            .into_iter()
            .map(|(k, v)| format!("{k}: {v}\r\n"))
            .flat_map(|s| s.into_bytes())
            .collect::<Vec<_>>();
        let headers = [headers, b"\r\n\r\n".to_vec()].concat();
        let response = format!("HTTP/1.1 {}\r\n\r\n", self.status);
        [response.as_bytes(), &headers, &self.body].concat()
    }
}

impl From<Response> for &[u8] {
    fn from(value: Response) -> Self {
        todo!()
    }
}

#[derive(Debug)]
pub enum Status {
    Ok,
    InternalServer,
    NotFound,
    MethodNotAllowed,
    BadRequest,
}

impl From<&Status> for &str {
    fn from(val: &Status) -> Self {
        use Status::*;

        match val {
            Ok => "200 OK",
            InternalServer => "500 Internal Server Error",
            NotFound => "404 Not Found",
            MethodNotAllowed => "405 Method Not Allowed",
            BadRequest => "400 Bad Request",
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status: &'static str = self.into();
        write!(f, "{}", status)
    }
}
