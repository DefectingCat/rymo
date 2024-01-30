use std::fmt::Display;

#[derive(Debug)]
pub enum Status {
    Ok,
    InternalServer,
}

impl From<&Status> for &str {
    fn from(val: &Status) -> Self {
        use Status::*;

        match val {
            Ok => "200 OK",
            InternalServer => "500 Internal Server Error",
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status: &'static str = self.into();
        write!(f, "{}", status)
    }
}
