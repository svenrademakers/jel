mod file_service;
mod match_service;
mod session_mananger;
use std::fmt::Display;

use async_trait::async_trait;
pub use file_service::*;
use hyper::Body;
pub use match_service::*;
pub use session_mananger::*;

#[async_trait]
pub trait RequestHandler: Send + Sync + 'static + Display {
    async fn invoke(&self, request: http::Request<Body>) -> std::io::Result<http::Response<Body>>;
}
