use std::{
    collections::HashMap,
    ffi::OsStr,
    hash::Hash,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use chrono::{serde::ts_seconds, DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct Recording {
    name: String,
    sources: Vec<Source>,
}

#[derive(Serialize)]
pub struct Source {
    typ: StreamingType,
    #[serde(with = "ts_seconds")]
    created: DateTime<Utc>,
    url: PathBuf,
}

#[derive(Serialize)]
pub enum StreamingType {
    HLS,
    DASH,
}

#[async_trait]
pub trait Recordings: Send + Sync + 'static {
    async fn get_all(&self) -> Vec<Recording>;
}

pub struct RecordingsOnDisk {
    recording_map: HashMap<String, Vec<Source>>,
}

impl RecordingsOnDisk {
    pub async fn new(root: PathBuf) -> Self {
        let recording_map = scan_filesystem(&root).await;
        RecordingsOnDisk {
            recording_map: HashMap::new(),
        }
    }
}

#[async_trait]
impl Recordings for RecordingsOnDisk {
    async fn get_all(&self) -> Vec<Recording> {
        Vec::new()
    }
}

async fn scan_filesystem(root: &Path) -> HashMap<String, Vec<Source>> {
    let mut found: HashMap<String, Vec<Source>> = HashMap::new();
    let mut dir_entry = tokio::fs::read_dir(root).await.unwrap();
    while let Ok(Some(entry)) = dir_entry.next_entry().await {
        let file = PathBuf::from(entry.file_name());
        let typ = match file.extension().and_then(OsStr::to_str) {
            Some("m3u8") => StreamingType::HLS,
            _ => continue,
        };

        let url = match entry.path().strip_prefix(root) {
            Ok(path) => path.to_path_buf(),
            Err(_) => continue,
        };

        let created = match entry.metadata().await.and_then(|meta| meta.created()) {
            Ok(time) => time,
            Err(_) => continue,
        };

        let source = Source {
            typ,
            url: url.to_path_buf(),
            created: created.into(),
        };

        if let Some(stem) = file.file_stem() {
            found
                .entry(stem.to_string_lossy().to_string())
                .or_default()
                .push(source);
        }
    }
    found
}
