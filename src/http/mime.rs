// https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types/Common_types
pub static HTML_UTF_8: &str = "text/html; charset=utf-8";
pub static CSS_UTF_8: &str = "text/css; charset=utf-8";
pub static TEXT: &str = "text/plain";

pub fn read_mime(filename: &str) -> &'static str {
    match filename {
        "html" => HTML_UTF_8,
        "css" => CSS_UTF_8,
        _ => TEXT,
    }
}
