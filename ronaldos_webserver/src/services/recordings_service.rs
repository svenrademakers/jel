use std::{fmt::Display, sync::Arc};

use crate::middleware::interface::StreamStore;

use super::as_json_response;
use async_trait::async_trait;
use hyper_rusttls::service::RequestHandler;

pub struct RecordingsService {
    stream_store: Arc<dyn StreamStore>,
}

#[async_trait]
impl RequestHandler for RecordingsService {
    async fn invoke(
        &self,
        request: http::Request<hyper::Body>,
    ) -> std::io::Result<http::Response<hyper::Body>> {
        match request.uri().query().unwrap_or_default() {
            "untagged" => as_json_response(&self.stream_store.get_untagged_sources().await),
            _ => as_json_response(&self.stream_store.get_fixtures("fixtures").await),
        }
    }

    fn path() -> &'static str {
        "/streams"
    }
}

impl RecordingsService {
    pub fn new(stream_store: Arc<dyn StreamStore>) -> Self {
        RecordingsService { stream_store }
    }
}

impl Display for RecordingsService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "match recordings")
    }
}
