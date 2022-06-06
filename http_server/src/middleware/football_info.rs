use async_trait::async_trait;
use chrono::{serde::ts_seconds, DateTime, Utc};
use serde::{Deserialize, Serialize};

#[Serialize]
pub struct Fixture {
    fixture_id: u32,
    score: String,
    home: String,
    away: String,
    venue: String,
    #[serde(with = "ts_seconds")]
    date: DateTime<Utc>,
}

#[async_trait]
pub trait FootballInfo: Send + Sync + 'static {
    async fn match_info(fixture_id: u32) -> Option<Fixture>;
}
