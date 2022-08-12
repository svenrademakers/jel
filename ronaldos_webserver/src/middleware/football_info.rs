use async_trait::async_trait;
use chrono::{serde::ts_seconds, DateTime, Utc};
use http::{Request, Uri};
use hyper::{Body, Client};
use hyper_rusttls::https_connector::HttpsConnector;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::BTreeMap, error::Error, str::FromStr, sync::Arc};
use tokio::sync::RwLock;

use super::interface::StreamStore;

#[derive(Serialize, Deserialize, Clone)]
pub struct Fixture {
    pub fixture_id: u32,
    score: String,
    home: String,
    away: String,
    venue: String,
    #[serde(with = "ts_seconds")]
    timestamp: DateTime<Utc>,
}

#[async_trait]
pub trait FootballInfo: Send + Sync {
    async fn fixtures(&self, season: &str) -> Vec<Fixture>;
}

#[allow(dead_code)]
pub struct FootballApi {
    data: RwLock<BTreeMap<String, Fixture>>,
    url: http::uri::Uri,
    api_key: String,
    recordings: Arc<dyn StreamStore>,
}

impl FootballApi {
    pub async fn new(
        season: &str,
        team: &str,
        api_key: String,
        recordings: Arc<dyn StreamStore>,
    ) -> Self {
        let api_uri = http::Uri::from_str(&format!(
            "https://api-football-v1.p.rapidapi.com/v3/fixtures?season={}&team={}",
            season, team
        ))
        .unwrap();

        let raw = load_from_football_api(&api_uri, &api_key)
            .await
            .unwrap_or_default();
        let data = load(&raw).await.unwrap_or_default();

        FootballApi {
            data: RwLock::new(data),
            url: api_uri,
            api_key,
            recordings,
        }
    }
}

#[async_trait]
impl FootballInfo for FootballApi {
    async fn fixtures(&self, season: &str) -> Vec<Fixture> {
        if season == "2022" {
            self.data.read().await.values().cloned().collect()
        } else {
            Vec::new()
        }
    }
}

async fn load(str: &str) -> Result<BTreeMap<String, Fixture>, Box<dyn Error>> {
    let json: serde_json::Value = serde_json::from_str(&str)?;
    if let Some(msg) = json.get("message") {
        error!("{}", msg);
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "",
        )));
    }

    let mut fixtures = BTreeMap::new();
    for fixt in json["response"].as_array().unwrap() {
        let score;
        if fixt["goals"]["home"] == json!(null) || fixt["goals"]["away"] == json!(null) {
            score = "".to_string();
        } else {
            score = format!("{} - {}", fixt["goals"]["home"], fixt["goals"]["away"]);
        }

        let match_entry = json! {{
            "home" : fixt["teams"]["home"]["name"],
            "away" : fixt["teams"]["away"]["name"],
            "venue" : fixt["teams"]["away"]["name"],
            "score" : score,
            "timestamp" : fixt["fixture"]["timestamp"],
            "fixture_id" : fixt["fixture"]["id"],

        }};
        fixtures.insert(fixt["fixture"]["id"].to_string(), match_entry);
    }

    Ok(fixtures
        .into_iter()
        .map(|(x, y)| (x, serde_json::from_value::<Fixture>(y).unwrap()))
        .collect())
}

async fn load_from_football_api(url: &Uri, api_key: &str) -> Result<String, Box<dyn Error>> {
    debug!("downloading match data from football-api");
    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);
    let request = Request::builder()
        .method(hyper::Method::GET)
        .uri(url)
        .header("X-RapidAPI-Host", "api-football-v1.p.rapidapi.com")
        .header("X-RapidAPI-Key", api_key)
        .body(Body::empty())
        .unwrap();
    let res = client.request(request).await?;
    let bytes = hyper::body::to_bytes(res.into_body()).await?;
    let str = std::str::from_utf8(&bytes[..])?;
    Ok(str.to_string())
}
