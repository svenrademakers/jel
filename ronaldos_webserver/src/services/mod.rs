mod file_service;
mod fixture_service;
mod session_mananger;
mod stream_service;

use std::{ffi::OsStr, path::Path};

pub use file_service::*;
pub use fixture_service::*;
use http::Request;
use hyper::Body;
use serde::Serialize;
pub use session_mananger::*;
pub use stream_service::*;

fn lookup_content_type(path: &Path) -> Option<&'static str> {
    let content_type = match path.extension().and_then(OsStr::to_str) {
        Some("jpeg") => Some("image/jpeg"),
        Some("png") => Some("image/png"),
        Some("svg") => Some("image/svg+xml"),
        Some("json") => Some("application/json"),
        Some("js") => Some("text/javascript"),
        Some("css") => Some("text/css"),
        Some("html" | "htm") => Some("text/html; charset=UTF-8"),
        Some("m3u8") => Some("application/x-mpegURL"),
        Some("mp4") => Some("video/mp4"),
        _ => None,
    };
    content_type
}

pub fn first_segment_uri(request: &Request<Body>) -> Option<&str> {
    request.uri().path()[1..].split_terminator('/').next()
}

pub fn as_json_response<T>(value: &T) -> std::io::Result<http::Response<Body>>
where
    T: Serialize + ?Sized,
{
    let as_string = serde_json::to_string(value)?;

    Ok(http::Response::builder()
        .status(http::StatusCode::OK)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::CONTENT_LENGTH, as_string.len())
        .body(hyper::Body::from(as_string))
        .unwrap())
}
