use std::{fmt::Display, sync::Arc};

use async_trait::async_trait;
use hyper_rusttls::service::RequestHandler;

use crate::middleware::Recordings;

use super::as_json_response;

pub struct RecordingsService<T>
where
    T: Recordings,
{
    recordings: Arc<T>,
}

#[async_trait]
impl<T> RequestHandler for RecordingsService<T>
where
    T: Recordings,
{
    async fn invoke(
        &self,
        _: http::Request<hyper::Body>,
    ) -> std::io::Result<http::Response<hyper::Body>> {
        as_json_response(self.recordings.get_all().await)
    }

    fn path() -> &'static str {
        "/recordings"
    }
}

impl<T> RecordingsService<T>
where
    T: Recordings,
{
    pub fn new(recordings: Arc<T>) -> Self {
        RecordingsService { recordings }
    }
}

impl<T> Display for RecordingsService<T>
where
    T: Recordings,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "match recordings")
    }
}
