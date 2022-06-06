use std::fmt::Display;

use async_trait::async_trait;
use hyper_rusttls::service::RequestHandler;

use crate::middleware::{FootballInfo, Recordings};

pub struct RecordingsService<T, F>
where
    T: Recordings,
    F: FootballInfo,
{
    recordings: T,
    football_info: F,
}

#[async_trait]
impl<T, F> RequestHandler for RecordingsService<T, F>
where
    T: Recordings,
    F: FootballInfo,
{
    async fn invoke(
        &self,
        request: http::Request<hyper::Body>,
    ) -> std::io::Result<http::Response<hyper::Body>> {
        let string = serde_json::to_string(&self.recordings.get_all().await)?;

        Ok(http::Response::builder()
            .status(http::StatusCode::OK)
            .header(http::header::CONTENT_TYPE, "application/json")
            .header(http::header::CONTENT_LENGTH, string.len())
            .body(hyper::Body::from(string))
            .unwrap())
    }

    fn path() -> &'static str {
        "/recordings"
    }
}

impl<T, F> RecordingsService<T, F>
where
    T: Recordings,
    F: FootballInfo,
{
    pub fn new(recordings: T, football_info: F) -> Self {
        RecordingsService {
            recordings,
            football_info,
        }
    }
}

impl<T, F> Display for RecordingsService<T, F>
where
    T: Recordings,
    F: FootballInfo,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "match recordings")
    }
}
