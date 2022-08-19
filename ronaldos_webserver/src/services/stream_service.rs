use super::{as_json_response, lookup_content_type};
use crate::middleware::{data_types::StreamId, LocalStreamStore};
use async_trait::async_trait;
use hyper::Body;
use hyper_rusttls::service::RequestHandler;
use log::info;
use std::{fmt::Display, ops::Deref, path::PathBuf, sync::Arc};

pub struct RecordingsService {
    stream_store: Arc<LocalStreamStore>,
    dev_mode: bool,
}

impl RecordingsService {
    pub fn new(stream_store: Arc<LocalStreamStore>, dev_mode: bool) -> Self {
        RecordingsService {
            stream_store,
            dev_mode,
        }
    }

    async fn test(&self) -> std::io::Result<http::Response<hyper::Body>> {
        self.stream_store
            .register(
                StreamId::FootballAPI(1234),
                vec![PathBuf::from("test1.dash"), PathBuf::from("test1.m3u8")],
                "this is a test".to_string(),
                chrono::Utc::now(),
            )
            .await
            .unwrap();

        Ok(http::response::Builder::new()
            .status(http::StatusCode::OK)
            .body(Body::empty())
            .unwrap())
    }
}

impl Display for RecordingsService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "match recordings")
    }
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
            Some("test") if self.dev_mode => self.test().await,
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
