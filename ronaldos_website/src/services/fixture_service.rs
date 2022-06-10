use async_trait::async_trait;
use hyper::Body;
use hyper_rusttls::service::RequestHandler;
use regex::Regex;
use serde_json::Value;
use std::{fmt::Display, sync::Arc};

use crate::middleware::{FootballInfo, Recordings};

use super::as_json_response;

pub struct FixtureService {
    football_info: Arc<dyn FootballInfo>,
    recordings: Arc<dyn Recordings>,
}

impl FixtureService {
    pub fn new(football_info: Arc<dyn FootballInfo>, recordings: Arc<dyn Recordings>) -> Self {
        FixtureService {
            football_info,
            recordings,
        }
    }
}

#[async_trait]
impl RequestHandler for FixtureService {
    async fn invoke(&self, request: http::Request<Body>) -> std::io::Result<http::Response<Body>> {
        let mut fixtures = self
            .football_info
            .fixtures("2022")
            .await
            .into_iter()
            .map(|x| (x.fixture_id.to_string(), serde_json::to_value(&x).unwrap()))
            .collect::<serde_json::Map<String, Value>>();

        let re = Regex::new(".*[0-9]+.*").unwrap();
        for stream in self.recordings.get_all().await {
            if let Some(fixture_id) = re
                .find(stream.url.file_stem().unwrap().to_str().unwrap())
                .map(|m| m.as_str())
            {
                if fixtures.contains_key(fixture_id) {
                    let attributes = fixtures[fixture_id].as_object_mut().unwrap();
                    attributes.insert("source".to_string(), serde_json::to_value(&stream).unwrap());
                }
            }
        }
        as_json_response(fixtures)
    }

    fn path() -> &'static str {
        "/fixtures"
    }
}

impl Display for FixtureService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fixture")
    }
}
