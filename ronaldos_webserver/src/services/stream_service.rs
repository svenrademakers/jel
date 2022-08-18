use super::{as_json_response, lookup_content_type};
use crate::middleware::{data_types::StreamId, LocalStreamStore};
use async_trait::async_trait;
use hyper::Body;
use hyper_rusttls::service::RequestHandler;
use log::info;
use std::{fmt::Display, ops::Deref, path::PathBuf, sync::Arc};

pub struct RecordingsService {
    stream_store: Arc<LocalStreamStore>,
}

fn preflight_response() -> http::Response<Body> {
    http::Response::builder()
        .status(http::StatusCode::NO_CONTENT)
        .header(http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(http::header::ACCESS_CONTROL_MAX_AGE, "1728000")
        .header(http::header::CONTENT_TYPE, "text/plain charset=UTF-8")
        .header(http::header::CONTENT_LENGTH, "0")
        .body(Body::empty())
        .unwrap()
}

#[async_trait]
impl RequestHandler for RecordingsService {
    async fn invoke(
        &self,
        request: http::Request<hyper::Body>,
    ) -> std::io::Result<http::Response<hyper::Body>> {
        if request.method() == http::Method::OPTIONS {
            return Ok(preflight_response());
        }
        match request.uri().path()[1..].split_terminator('/').nth(1) {
            Some("all") => as_json_response(&self.stream_store.get_all("streams").await),
            Some("untagged") => {
                as_json_response(&self.stream_store.get_untagged_sources("streams").await)
            }
            Some(file) => {
                let data = self.stream_store.get_source(file).await.unwrap();
                let mut response = http::response::Builder::new()
                    .header(http::header::CACHE_CONTROL, "no-cache")
                    .header(http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "'*' always")
                    .header(
                        http::header::ACCESS_CONTROL_EXPOSE_HEADERS,
                        "Content-Length",
                    )
                    .header(http::header::CONTENT_LENGTH, data.len());

                if let Some(content_type) = lookup_content_type(file.as_ref()) {
                    response = response.header(http::header::CONTENT_TYPE, content_type);
                }

                Ok(response.body(Body::from(data.deref().clone())).unwrap())
            }
            None => Ok(http::Response::builder()
                .status(http::StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap()),
        }
    }

    fn path() -> &'static str {
        "streams"
    }
}

impl RecordingsService {
    pub fn new(stream_store: Arc<LocalStreamStore>) -> Self {
        RecordingsService { stream_store }
    }
}

impl Display for RecordingsService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "match recordings")
    }
}
