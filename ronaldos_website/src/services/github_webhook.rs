use std::fmt::Display;

use async_trait::async_trait;
use hyper_rusttls::service::RequestHandler;

struct GithubWebhook {}

#[async_trait]
impl RequestHandler for GithubWebhook {
    #[allow(unused_variables)]
    async fn invoke(
        &self,
        request: http::Request<hyper::Body>,
    ) -> std::io::Result<http::Response<hyper::Body>> {
        todo!()
    }

    fn path() -> &'static str {
        "/webhook"
    }
}

impl Display for GithubWebhook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Github webhook")
    }
}
