use async_trait::async_trait;
use hyper::Body;
use hyper_rusttls::service::RequestHandler;
use std::{fmt::Display, sync::Arc};

use crate::middleware::{FootballApi, LocalStreamStore};

#[allow(dead_code)]
pub struct FixtureService {
    football_info: Arc<FootballApi>,
    recordings: Arc<LocalStreamStore>,
}

impl FixtureService {
    pub fn new(football_info: Arc<FootballApi>, recordings: Arc<LocalStreamStore>) -> Self {
        FixtureService {
            football_info,
            recordings,
        }
    }
}

#[async_trait]
impl RequestHandler for FixtureService {
    async fn invoke(&self, _: http::Request<Body>) -> std::io::Result<http::Response<Body>> {
        let mut data = Vec::new();
        self.football_info.fixtures(&mut data).await.unwrap();

        Ok(http::Response::builder()
            .status(http::StatusCode::OK)
            .header(http::header::CONTENT_TYPE, "application/json")
            .header(http::header::CONTENT_LENGTH, data.len())
            .body(hyper::Body::from(data))
            .unwrap())
    }

    fn path() -> &'static str {
        "fixtures"
    }
}

impl Display for FixtureService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fixture")
    }
}
