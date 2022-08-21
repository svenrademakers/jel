use chrono::{serde::ts_seconds, DateTime, Utc};
use log::error;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf, StripPrefixError},
};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub(super) struct MetaFile {
    // Stream can be in different formats and resolutions. Usually we make use
    // of one "master playlist" so typically only one source is ossociated with
    // a stream.
    pub filenames: Vec<PathBuf>,
    // additional field to specify a custom title
    pub description: String,
    #[serde(with = "ts_seconds")]
    pub date: DateTime<Utc>,
    pub live : Option<bool>,
}

impl From<MetaFile> for Stream {
    fn from(meta: MetaFile) -> Self {
        let mut sources = Vec::new();
        let mut live = false;
        for path in meta.filenames {
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
            sources,
            live : meta.live.unwrap_or_default(),
            description: meta.description,
            date: meta.date,
        }
    }
}

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq)]
pub struct Source {
    pub url: PathBuf,
    pub typ: String,
}

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq)]
pub struct Stream {
    pub sources: Vec<Source>,
    pub description: String,
    #[serde(with = "ts_seconds")]
    pub date: DateTime<Utc>,
    pub live: bool,
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
