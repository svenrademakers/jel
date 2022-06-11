use std::{
    collections::HashMap,
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use chrono::{serde::ts_seconds, DateTime, Utc};
use log::warn;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Serialize, Deserialize, Clone)]
pub struct Source {
    pub url: PathBuf,
    typ: StreamingType,
    #[serde(with = "ts_seconds")]
    created: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum StreamingType {
    HLS,
    DASH,
}

#[async_trait]
pub trait Recordings: Send + Sync {
    async fn get_all(&self) -> Vec<Source>;
    async fn register<T>(&self, cb: &T)
    where
        T: Fn((Source, bool)) -> bool + Send + Sync,
        Self: Sized;
}

pub type UpdateCallback = dyn Fn((Source, bool)) -> bool + Send + Sync;
type ObserverList = RwLock<Vec<Arc<UpdateCallback>>>;

pub struct RecordingsOnDisk {
    recording_map: RwLock<HashMap<PathBuf, Source>>,
    root: PathBuf,
    observers: ObserverList,
}

impl RecordingsOnDisk {
    pub async fn new(root: PathBuf) -> Self {
        if !root.exists() {
            warn!("creating {}, does not exist", root.to_string_lossy());
            tokio::fs::create_dir_all(&root).await.unwrap();
        } 
        
        let recording_map = scan_filesystem(&root, Some(&root))
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|s| (s.url.clone(), s))
            .collect();

        RecordingsOnDisk {
            recording_map: RwLock::new(recording_map),
            root,
            observers: RwLock::new(Vec::new()),
        }
    }
}

#[async_trait]
impl Recordings for RecordingsOnDisk {
    async fn get_all(&self) -> Vec<Source> {
        self.recording_map.read().await.values().cloned().collect()
    }

    async fn register<T>(&self, cb: &T)
    where
        T: Fn((Source, bool)) -> bool + Send + Sync,
    {
        todo!()
    }
}

async fn scan_filesystem(root: &Path, path: Option<&Path>) -> Option<Vec<Source>> {
    let mut found: Vec<Source> = Vec::new();
    if !path.map_or_else(|| false, |p| p.starts_with(root)) {
        return None;
    }

    let mut dir_entry = tokio::fs::read_dir(path.unwrap_or(root)).await.unwrap();
    while let Ok(Some(entry)) = dir_entry.next_entry().await {
        let file = PathBuf::from(entry.file_name());
        let typ = match file.extension().and_then(OsStr::to_str) {
            Some("m3u8") => StreamingType::HLS,
            Some("dash") => StreamingType::DASH,
            _ => continue,
        };

        let url = match entry.path().strip_prefix(root.parent().unwrap()) {
            Ok(path) => path.to_path_buf(),
            Err(_) => continue,
        };

        let created = entry
            .metadata()
            .await
            .and_then(|meta| meta.created())
            .unwrap();

        let source = Source {
            typ,
            url: url.to_path_buf(),
            created: created.into(),
        };

        found.push(source);
    }

    Some(found)
}
