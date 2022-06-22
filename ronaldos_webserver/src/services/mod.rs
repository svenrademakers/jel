mod file_service;
mod fixture_service;
mod github_webhook;
mod recordings_service;
mod session_mananger;

pub use file_service::*;
pub use fixture_service::*;
use hyper::Body;
pub use recordings_service::*;
use serde::Serialize;
pub use session_mananger::*;

pub fn as_json_response<T>(value: T) -> std::io::Result<http::Response<Body>>
where
    T: Serialize,
{
    let as_string = serde_json::to_string(&value)?;

    Ok(http::Response::builder()
        .status(http::StatusCode::OK)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::CONTENT_LENGTH, as_string.len())
        .body(hyper::Body::from(as_string))
        .unwrap())
}
