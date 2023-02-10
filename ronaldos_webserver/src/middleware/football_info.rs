use actix_tls::connect::rustls::webpki_roots_cert_store;
use actix_web::http::{self, Uri};
use anyhow::{Context, Result};
use bytes::Bytes;
use chrono::{serde::ts_seconds, DateTime, Utc};
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
    pub async fn new(season: &str, team: &str, api_key: String) -> Self {
        let api_uri = http::Uri::from_str(&format!(
            "https://api-football-v1.p.rapidapi.com/v3/fixtures?season={}&team={}",
            season, team
        ))
        .unwrap();

        FootballApi {
            cache: RwLock::new(BTreeMap::new()),
            url: api_uri,
            api_key,
        }
    }

    pub async fn fixtures<T: Write>(&self, writer: &mut T) -> Result<()> {
        let mut write_cache = self.cache.write().await;
        // load cache on first request
        if write_cache.is_empty() {
            if self.api_key.is_empty() {
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

async fn to_data_model(json: serde_json::Value) -> Result<BTreeMap<String, Vec<Value>>> {
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

async fn football_api_request(url: &Uri, api_key: &str) -> anyhow::Result<serde_json::Value> {
    debug!("downloading match data from football-api");
    let config = rustls::client::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(webpki_roots_cert_store())
        .with_no_client_auth();
    let client = awc::Client::builder()
        .connector(awc::Connector::new().rustls(Arc::new(config)))
        .finish();
    let request = client
        .get(url)
        .insert_header(("X-RapidAPI-Host", "api-football-v2.p.rapidapi.com"))
        .insert_header(("X-RapidAPI-Key", api_key));
    let mut res = request.send().await.unwrap();
    res.json::<serde_json::Value>()
        .await
        .context("not a valid json reponse body")
}
