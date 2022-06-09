use std::fmt::Display;

use async_trait::async_trait;
use hyper::Body;

#[async_trait]
pub trait RequestHandler: Send + Sync + Display {
    async fn invoke(&self, request: http::Request<Body>) -> std::io::Result<http::Response<Body>>;
    fn path() -> &'static str
    where
        Self: Sized;
}
