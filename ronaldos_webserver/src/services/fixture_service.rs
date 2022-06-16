use async_trait::async_trait;
use hyper::Body;
use hyper_rusttls::service::RequestHandler;
use serde_json::Value;
use std::{collections::BTreeMap, fmt::Display, sync::Arc};

use crate::middleware::{interface::StreamStore, FootballInfo};

use super::as_json_response;

pub struct FixtureService {
    football_info: Arc<dyn FootballInfo>,
    recordings: Arc<dyn StreamStore>,
}

impl FixtureService {
    pub fn new(football_info: Arc<dyn FootballInfo>, recordings: Arc<dyn StreamStore>) -> Self {
        FixtureService {
            football_info,
            recordings,
        }
    }
}

#[async_trait]
impl RequestHandler for FixtureService {
    async fn invoke(&self, _: http::Request<Body>) -> std::io::Result<http::Response<Body>> {
        let mut fixtures = self
            .football_info
            .fixtures("2022")
            .await
            .into_iter()
            .map(|x| (x.fixture_id, serde_json::to_value(&x).unwrap()))
            .collect::<BTreeMap<u32, Value>>();

        for stream in self.recordings.get_fixtures(FixtureService::path()).await {
            if let Some(Some(attributes)) = fixtures
                .get_mut(&stream.0)
                .map(serde_json::Value::as_object_mut)
            {
                attributes.insert("source".to_string(), serde_json::to_value(&stream).unwrap());
            }
        }
        as_json_response(&fixtures)
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
