use chrono::{serde::ts_seconds, DateTime, Utc};
use log::{debug, error};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf, StripPrefixError},
};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MetaFile {
    // unique id to identify stream. assumed can be that id == football-api
    // fixture id. otherwise id depicts a custom stream
    pub id: StreamId,
    // stream can be in different formats and resolutions.
    pub filenames: Vec<PathBuf>,
    // will be set to true explicitly by streaming encoder if the stream is
    // actually live.
    pub live: Option<bool>,
    // additional field to specify a custom title
    pub title: Option<PathBuf>,
}

impl MetaFile {
    pub async fn into_metadata(self, root: &Path) -> Option<(StreamId, Stream)> {
        let mut sources = Vec::new();
        for path in self.filenames {
            let mut created = std::time::UNIX_EPOCH;
            match tokio::fs::metadata(root.join(&path))
                .await
                .and_then(|meta| meta.created())
            {
                Ok(time) => created = time,
                Err(ref e) if e.kind() == std::io::ErrorKind::Unsupported => {
                    debug!("created call not supported by platform");
                }
                Err(e) => error!("error retrieving metadata {}", e),
            }
            let typ = match path.extension().and_then(OsStr::to_str) {
                Some("m3u8") => StreamingType::HLS,
                Some("dash" | "mpd") => StreamingType::DASH,
                _ => return None,
            };

            sources.push(Source {
                typ,
                url: path.to_path_buf(),
                created: created.into(),
            });
        }
        Some((
            self.id,
            Stream {
                sources,
                live: self.live.unwrap_or_default(),
            },
        ))
    }
}

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq)]
pub struct Source {
    pub url: PathBuf,
    pub typ: StreamingType,
    #[serde(with = "ts_seconds")]
    pub created: DateTime<Utc>,
}

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq)]
pub enum StreamingType {
    HLS,
    DASH,
}

#[derive(Default, Serialize, Debug, Deserialize, Clone, PartialEq)]
pub struct Stream {
    pub sources: Vec<Source>,
    pub live: bool,
}

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum StreamId {
    FootballAPI(u32),
    Untagged(u32),
    None,
}

impl StreamId {
    const INVALID_KEY: u32 = 0;
    pub fn get_raw_key(&self) -> Option<u32> {
        let key = match self {
            StreamId::FootballAPI(key) => key,
            StreamId::Untagged(key) => key,
            StreamId::None => return None,
        };
        Some(*key)
    }
}

impl Default for StreamId {
    fn default() -> Self {
        StreamId::Untagged(StreamId::INVALID_KEY)
    }
}

impl Stream {
    pub fn strip_prefix(&mut self, prefix: &Path) -> Result<(), StripPrefixError> {
        for source in self.sources.iter_mut() {
            match source.url.strip_prefix(prefix) {
                Ok(path) => source.url = path.to_path_buf(),
                Err(e) => {
                    error!(
                        "path {} not in root {}",
                        source.url.to_string_lossy(),
                        prefix.to_string_lossy()
                    );
                    return Err(e);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum RegisterError {
    IdAlreadyRegisteredTo(Stream),
    SourceArgumentEmpty,
    ParseError(serde_yaml::Error),
    IoError(std::io::Error),
}
