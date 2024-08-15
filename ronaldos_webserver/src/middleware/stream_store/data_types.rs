use chrono::{serde::ts_seconds, DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{ffi::OsStr, path::PathBuf};
use thiserror::Error;
use uuid::Uuid;

pub type MetaFile = StreamMeta<PathBuf>;
pub type Stream = StreamMeta<Source>;
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StreamMeta<T>
where
    T: Clone + PartialEq + Eq,
{
    pub uuid: Uuid,
    pub sources: Vec<T>,
    pub description: String,
    #[serde(with = "ts_seconds")]
    pub date: DateTime<Utc>,
    pub live: Option<bool>,
}

impl From<StreamMeta<PathBuf>> for StreamMeta<Source> {
    fn from(meta: StreamMeta<PathBuf>) -> Self {
        let mut sources = Vec::new();
        for path in meta.sources {
            let typ = match path.extension().and_then(OsStr::to_str) {
                Some("m3u8" | "m3u") => "application/x-mpegURL",
                Some("dash" | "mpd") => "application/dash+xml",
                Some("mp4") => "video/mp4",
                x => panic!("cannot map {:?} to MIME type", x),
            };
            sources.push(Source {
                typ: typ.into(),
                url: path,
            });
        }

        Stream {
            uuid: meta.uuid,
            sources,
            live: meta.live,
            description: meta.description,
            date: meta.date,
        }
    }
}

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct Source {
    pub url: PathBuf,
    pub typ: String,
}

#[derive(Debug, Error)]
pub enum RegisterError {
    #[error("no source url specified")]
    SourceArgumentEmpty,
    #[error(transparent)]
    ParseError(#[from] serde_yaml::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}
