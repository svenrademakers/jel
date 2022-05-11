use crate::tls::HttpsConnector;

use super::RequestHandler;
use async_trait::async_trait;
use hyper::client::Client;
use hyper::{Body, Request};
use log::{debug, info, trace};
use serde_json::json;
use std::error::Error;
use std::fmt::Display;
use tokio::sync::RwLock;

#[derive(Debug, Default)]
pub struct MatchService {
    data: RwLock<Vec<u8>>,
    url: http::uri::Uri,
}

#[async_trait]
impl RequestHandler for MatchService {
    async fn invoke(&self, _: http::Request<Body>) -> std::io::Result<http::Response<Body>> {
        let data = self.get_matches_slice().await;
        Ok(http::Response::builder()
            .status(http::StatusCode::OK)
            .header(http::header::CONTENT_TYPE, "application/json")
            .header(http::header::CONTENT_LENGTH, data.len())
            .body(data.into())
            .unwrap())
    }
}

impl MatchService {
    pub fn new(season: &str, team: &str) -> Self {
        let api_uri = format!(
            "https://api-football-v1.p.rapidapi.com/v3/fixtures?season={}&team={}",
            season, team
        );
        MatchService {
            data: RwLock::new(Vec::new()),
            url: http::Uri::try_from(api_uri).unwrap(),
        }
    }

    pub async fn refresh(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        debug!("downloading match data from football-api");
        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, hyper::Body>(https);

        let request = Request::builder()
            .method(hyper::Method::GET)
            .uri(self.url.clone())
            .header("X-RapidAPI-Host", "api-football-v1.p.rapidapi.com")
            .header(
                "X-RapidAPI-Key",
                std::env::var("API_KEY").unwrap_or_default(),
            )
            .body(Body::empty())
            .unwrap();
        let res = client.request(request).await?;
        let bytes = hyper::body::to_bytes(res.into_body()).await?;
        let str = std::str::from_utf8(&bytes[..])?;
        trace!("RECV {}", str);
        let json: serde_json::Value = serde_json::from_str(str)?;

        let mut fixtures = serde_json::Map::new();
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

            }};
            fixtures.insert(fixt["fixture"]["id"].to_string(), match_entry);
        }
        Ok(serde_json::to_vec(&fixtures)?)
    }

    async fn get_matches_slice(&self) -> Vec<u8> {
        match self.data.read().await.is_empty() {
            true => {
                let new_data = self.refresh().await.unwrap();
                *self.data.write().await = new_data.clone();
                new_data
            }
            false => self.data.read().await.clone(),
        }
    }
}

impl Display for MatchService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MatchService")
    }
}
