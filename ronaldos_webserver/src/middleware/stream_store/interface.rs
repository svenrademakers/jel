use std::{
    collections::BTreeMap,
    fmt::Debug,
    path::{Path, PathBuf, StripPrefixError},
};

use async_trait::async_trait;
use chrono::{serde::ts_seconds, DateTime, Utc};
use log::error;
use serde::{Deserialize, Serialize};

/// this trait describes functionality to get video streams that are persisted
/// by the implementor of this interface. This trait does not give any
/// guarantuee that data survives reboots or to what degree or how long videos
/// are stored. Refer to the implementations of this trait for more information
///
/// # About Streams
///
/// Currently 2 types of streams exist:
/// 1. Football fixture related. These streams correspond to an actual football
///    fixture.
/// 2. Untagged. These are streams that do not direct relate to an actual
///    fixture, but can be any content.
///
/// Football streams have the benifit in that they can be correlated with other
/// football information and systems. The actual key is dictated by an external
/// trait, see [super::FootballInfo].
///
/// you can assume the following about stream data:
/// * [StreamId] defines the unique key to index an stream
/// * [StreamId::FootballAPI] should reference a valid `fixture_id`
/// * a [Stream] can contain multiple sources in multiple formats. This is to
///   offer viewers compatibility and the choice to throttle different
///   qualities. All sources show the same content!
#[async_trait]
pub trait StreamStore: Send + Sync {
    async fn get_fixtures(&self, prefix: &'static str) -> BTreeMap<u32, Stream>;

    /// registers a new fixture
    async fn register(
        &self,
        id: StreamId,
        sources: Vec<PathBuf>,
        live: bool,
        title: Option<PathBuf>,
    ) -> Result<(), RegisterError>;

    async fn get_untagged_sources(&self) -> Vec<PathBuf>;
}

#[derive(Debug)]
pub enum RegisterError {
    IdAlreadyRegisteredTo(Stream),
    SourceArgumentEmpty,
    ParseError(serde_yaml::Error),
    IoError(std::io::Error),
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

    pub fn is_invalid(&self) -> bool {
        let key = match self {
            StreamId::FootballAPI(key) => key,
            StreamId::Untagged(key) => key,
            StreamId::None => return false,
        };

        key == &StreamId::INVALID_KEY
    }

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
