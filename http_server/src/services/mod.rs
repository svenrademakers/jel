mod get_file;
mod match_service;
mod session_mananger;
use async_trait::async_trait;
pub use get_file::*;
use hyper::Body;
pub use match_service::*;
pub use session_mananger::*;

#[async_trait]
pub trait RequestHandler: Send + Sync + 'static {
    async fn invoke(&self, request: http::Request<Body>) -> std::io::Result<http::Response<Body>>;
}
