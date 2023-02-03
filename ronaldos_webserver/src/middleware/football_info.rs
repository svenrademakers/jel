use anyhow::{Context, Result};
use bytes::Bytes;
use chrono::{serde::ts_seconds, DateTime, Utc};
use http::{Request, Uri};
use hyper::{body, Body, Client};
use hyper_rusttls::https_connector::HttpsConnector;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, io::Write, str::FromStr, sync::Arc};
use tokio::sync::RwLock;

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
#[allow(dead_code)]
pub struct FootballApi {
    /// map of league name as key and Fixture as item
    cache: RwLock<BTreeMap<String, Vec<Value>>>,
    url: http::uri::Uri,
    api_key: String,
}

impl FootballApi {
    pub async fn new(season: &str, team: &str, api_key: String) -> Arc<Self> {
        let api_uri = http::Uri::from_str(&format!(
            "https://api-football-v1.p.rapidapi.com/v3/fixtures?season={}&team={}",
            season, team
        ))
        .unwrap();

        let instance = Arc::new(FootballApi {
            cache: RwLock::new(BTreeMap::new()),
            url: api_uri,
            api_key,
        });

        instance
    }

    pub async fn fixtures<T: Write>(&self, writer: &mut T) -> Result<()> {
        let mut write_cache = self.cache.write().await;
        // load cache on first request
        if write_cache.is_empty() {
            if self.api_key.is_empty(){
                info!("no football api key set. omitting fixture data");
                return Ok(());
            }
            debug!("cache not loaded yet, sending football request");
            let raw = football_api_request(&self.url, &self.api_key).await?;
            let mut map = to_data_model(raw).await?;
            write_cache.append(&mut map);
        }

        let str = serde_json::to_string(&*write_cache)?;
        writer.write_all(str.as_bytes())?;
        Ok(())
    }
}

async fn to_data_model(bytes: Bytes) -> Result<BTreeMap<String, Vec<Value>>> {
    let json: serde_json::Value = serde_json::from_slice(&bytes)?;

    let mut fixtures: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for fixt in json["response"]
        .as_array()
        .with_context(|| format!("response: {}", json))?
    {
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

        fixtures
            .entry(
                fixt["league"]["name"]
                    .as_str()
                    .expect("need a key")
                    .to_string(),
            )
            .or_default()
            .push(match_entry);
    }
    Ok(fixtures)
}

async fn football_api_request(url: &Uri, api_key: &str) -> Result<Bytes> {
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
    Ok(body::to_bytes(res.into_body()).await?)
}
