use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use log::{debug, error};
use serde::{Deserialize, Serialize};

use super::interface::{Source, Stream, StreamId, StreamingType};

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
