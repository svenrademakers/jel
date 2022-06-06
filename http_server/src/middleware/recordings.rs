use std::{
    collections::HashMap,
    ffi::OsStr,
    hash::Hash,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use chrono::{serde::ts_seconds, DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::{self, Receiver, Sender};
use tokio::sync::RwLock;

#[derive(Serialize)]
pub struct Source {
    url: PathBuf,
    typ: StreamingType,
    #[serde(with = "ts_seconds")]
    created: DateTime<Utc>,
}

#[derive(Serialize)]
pub enum StreamingType {
    HLS,
    DASH,
}

#[async_trait]
pub trait Recordings: Send + Sync + 'static {
    async fn get_all(&self) -> Vec<Source>;
    async fn register(&self, cb: Box<Fn((Source, bool)) -> bool>);
}

type observer_list = Arc<RwLock<Vec<Box<Fn(Source, bool) -> bool>>>>;

pub struct RecordingsOnDisk {
    recording_map: RwLock<HashMap<PathBuf, Source>>,
    root: PathBuf,
    observers: observer_list,
}

impl RecordingsOnDisk {
    pub async fn new(root: PathBuf) -> Self {
        let mut recording_map = scan_filesystem(&root)
            .await
            .unwrap_or_default()
            .iter()
            .map(|s| (s.url, s))
            .collect();

        RecordingsOnDisk {
            recording_map,
            root,
            observers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Recordings for RecordingsOnDisk {
    async fn get_all(&self) -> Vec<Source> {
        self.recording_map.values().collect()
    }

    async fn register(&self, cb: Box<Fn((Source, bool)) -> bool>) {
        self.observers.write().await.push(cb)
    }
}

async fn scan_filesystem(root: &Path, path: Option<&Path>) -> Option<Vec<Source>> {
    let mut found: Vec<Source> = Vec::new();
    if !path.map_or_else(false, |p| path.starts_with(root)) {
        return None;
    }

    let mut dir_entry = tokio::fs::read_dir(path.unwrap_or(root)).await.unwrap();
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

        let created = entry
            .metadata()
            .await
            .and_then(|meta| meta.created())
            .unwrap_or_default();

        let source = Source {
            typ,
            url: url.to_path_buf(),
            created: created.into(),
        };

        if let Some(stem) = file.file_stem() {
            found.push(source);
        }
    }

    Some(found)
}
