use super::{as_json_response, lookup_content_type};
use crate::middleware::LocalStreamStore;
use async_trait::async_trait;
use hyper::Body;
use hyper_rusttls::service::RequestHandler;
use std::{
    fmt::Display,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

pub struct StreamsService {
    stream_store: Arc<LocalStreamStore>,
    base_url: String,
    dev_mode: bool,
}

impl StreamsService {
    pub fn new<T: Into<String>>(stream_store: Arc<LocalStreamStore>, host: T, dev_mode: bool) -> Self {
        StreamsService {
            stream_store,
            base_url: format!("{}/streams",host.into()),
            dev_mode,
        }
    }

    async fn test(&self) -> std::io::Result<http::Response<hyper::Body>> {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let number = COUNTER.fetch_add(1, Ordering::SeqCst);

        let test_description = format!("this is a test {}", number);

        self.stream_store
            .register(
                test_description,
                vec![PathBuf::from("test1.dash"), PathBuf::from("test1.m3u8")],
                chrono::Utc::now(),
            )
            .await
            .unwrap();

        Ok(http::response::Builder::new()
            .status(http::StatusCode::OK)
            .body(Body::empty())
            .unwrap())
    }
fn preflight_response(&self) -> http::Response<Body> {
    http::Response::builder()
        .status(http::StatusCode::NO_CONTENT)
        .header(http::header::ACCESS_CONTROL_ALLOW_HEADERS, "Content-Length, Content-Type, Range")
        .header(http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(http::header::ACCESS_CONTROL_MAX_AGE, "1728000")
        .header(http::header::CONTENT_TYPE, "text/plain charset=UTF-8")
        .header(http::header::CONTENT_LENGTH, "0")
        .body(Body::empty())
        .unwrap()
}
}

impl Display for StreamsService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stream Service")
    }
}


#[async_trait]
impl RequestHandler for StreamsService {
    async fn invoke(
        &self,
        request: http::Request<hyper::Body>,
    ) -> std::io::Result<http::Response<hyper::Body>> {
        if request.method() == http::Method::OPTIONS {
            return Ok(self.preflight_response());
        }

        let cursor = request.uri().path()[1..].find('/').unwrap() + 2;
        match &request.uri().path()[cursor..] {
            "test" if self.dev_mode => self.test().await,
            "all" => as_json_response(&self.stream_store.get_available_streams(&self.base_url).await),
            file => {
                let data = self.stream_store.get_segment(file).await.unwrap();
                let mut response = http::response::Builder::new()
                    .header(http::header::CACHE_CONTROL, "no-cache")
                    .header(http::header::ACCEPT_ENCODING, "identity")
                    .header(http::header::ACCEPT_RANGES, "bytes")
                    .header(http::header::ACCESS_CONTROL_ALLOW_HEADERS, "*")
                    .header(http::header::ACCESS_CONTROL_ALLOW_METHODS, "POST, GET, OPTIONS")
                    .header(http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                    .header(http::header::ACCESS_CONTROL_MAX_AGE, "1728000")
                    .header(
                        http::header::ACCESS_CONTROL_EXPOSE_HEADERS,
                        "Content-Length",
                    )
                    .header(http::header::CONTENT_LENGTH, data.len());

                if let Some(content_type) = lookup_content_type(file.as_ref()) {
                    response = response.header(http::header::CONTENT_TYPE, content_type);
                }

                Ok(response.body(data.into()).unwrap())
            }
            _ => Ok(http::Response::builder()
                .status(http::StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap()),
        }
    }

    fn path() -> &'static str {
        "streams"
    }
}
