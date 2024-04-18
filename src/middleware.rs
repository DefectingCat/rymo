use crate::error::Result;
use crate::{request::Request, response::Response};

pub trait Middleware {
    fn next(req: Request, res: Response) -> Result<(Request, Response)>;
}
