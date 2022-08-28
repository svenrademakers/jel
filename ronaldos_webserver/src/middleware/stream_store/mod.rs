pub mod data_types;
mod file_watcher;

use self::data_types::*;
use super::cache_map::CacheController;
use anyhow::{bail, ensure, Context, Result};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use log::{debug, error, info, trace};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    hash::Hash,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Provides video streams that are persisted on the filesystem. Even given they
/// are written real-time. At the moment there is support for the following
/// formats:
/// * HLS
/// * DASH
/// * MP4
///
/// > note that the file size of MP4 files cannot exceed available RAM space, in
/// this case the program will crash.
///
/// # Ajustable Bitrate
///
/// Currently we assume that streams are provided in 3 levels of quality, to
/// accommodate most bandwidth capabilities of clients watching theses streams. To
/// accommodate for this as best as possible we have 3 caches for each level, so
/// we keep the amount of memory allocations at a minimum
pub struct LocalStreamStore {
    /// Directory which all stream files will be written to. All paths used in
    /// [StreamStoreImpl] are relative compared to the root directory
    root: PathBuf,
    /// Map that contains the index of found streams. This is the single source
    /// of truth.
    stream_map: RwLock<BTreeMap<Uuid, Stream>>,
    uuid_lookup: RwLock<BTreeMap<PathBuf, Uuid>>,
    /// 3 way cache, caching streams optimized for the 3 different bitrate levels.
    file_cache: RwLock<CacheController<Path, Bytes, 128>>,
    /// This watcher object is used to exit the watcher task.
    watcher_sender: Option<tokio::sync::oneshot::Sender<()>>,
}

impl LocalStreamStore {
    pub async fn new(root: &Path) -> Arc<LocalStreamStore> {
        if !root.exists() {
            info!("creating {}, does not exist", root.to_string_lossy());
            tokio::fs::create_dir_all(&root).await.unwrap();
        }

        Arc::new(LocalStreamStore {
            root: root.to_path_buf(),
            stream_map: RwLock::new(BTreeMap::default()),
            uuid_lookup: RwLock::new(BTreeMap::default()),
            file_cache: RwLock::new(CacheController::new()),
            watcher_sender: None,
        })
    }

    pub fn run(instance: &mut Arc<LocalStreamStore>) {
        let (kill_sender, kill_receiver) = tokio::sync::oneshot::channel();

        let value =
            Arc::get_mut(instance).expect("LocalStreamStore cannot be shared before run call");
        value.watcher_sender = Some(kill_sender);

        // spawn loading task
        let loading_instance = instance.clone();
        tokio::spawn(async move {
            loading_instance
                .load(loading_instance.root.to_path_buf())
                .await
                .unwrap();
        });

        // spawn file watcher task
        Self::watch_for_changes(instance.clone(), kill_receiver);
    }

    /// Load a .stream meta file from disk. path can be a directory or a file.
    /// note that recursive scanning is disabled. see [LocalStreamStore::scan]
    async fn load(&self, path: PathBuf) -> Result<()> {
        let mut lookup = Vec::new();
        let new_meta_files = self
            .scan(&path)
            .await?
            .map(|(p, meta)| {
                lookup.push((p, meta.uuid));
                (meta.uuid, meta.into())
            })
            .collect::<Vec<(Uuid, Stream)>>();

        self.uuid_lookup.write().await.extend(lookup);
        self.stream_map.write().await.extend(new_meta_files);
        Ok(())
    }

    /// Scans for .stream files in a given path none recursively. If path is not a
    /// sub directory of root, None is returned.
    async fn scan(&self, path: &Path) -> Result<impl Iterator<Item = (PathBuf, MetaFile)>> {
        ensure!(
            path.starts_with(&self.root),
            format!(
                "{} is not in {}",
                path.to_string_lossy(),
                self.root.to_string_lossy()
            )
        );

        trace!("scanning: {:?}", &path);
        let mut found = Vec::new();
        let mut push_found = |path: &Path| match self.parse_file(path) {
            Ok(tuple) => found.push(tuple),
            Err(e) => error!("{}", e),
        };

        let md = tokio::fs::metadata(path)
            .await
            .with_context(|| format!("failed to get metadata for {}", path.to_string_lossy()))?;
        if md.is_file() {
            push_found(path);
        } else {
            let mut dir_entry = tokio::fs::read_dir(path)
                .await
                .with_context(|| format!("failed to read dir {}", path.to_string_lossy()))?;
            while let Ok(Some(entry)) = dir_entry.next_entry().await {
                push_found(&entry.path());
            }
        }

        debug!(
            "found {} stream(s) in {}",
            found.len(),
            path.to_string_lossy()
        );

        Ok(found.into_iter())
    }

    fn parse_file(&self, path: &Path) -> Result<(PathBuf, MetaFile)> {
        if path.extension() != Some(OsStr::new("stream")) {
            bail!(
                "will not parse {}. not a .stream extension",
                path.to_string_lossy()
            );
        }

        let relative = path
            .strip_prefix(&self.root)
            .expect("root tested on the start of the function")
            .to_path_buf();
        trace!("scanning {}", relative.to_string_lossy());

        let file = std::fs::File::open(path)
            .with_context(|| format!("error opening {}", path.to_string_lossy()))?;

        let stream = serde_yaml::from_reader::<std::fs::File, MetaFile>(file)
            .with_context(|| format!("could not parse {}", path.to_string_lossy()))?;
        Ok((path.to_path_buf(), stream))
    }

    pub async fn removed(&self, path: PathBuf) -> bool {
        if path.extension() != Some(OsStr::new("stream")) {
            return false;
        }

        let lookup = self.uuid_lookup.read().await;
        let uuid = match lookup.get(&path) {
            Some(val) => val,
            None => {
                return false;
            }
        };

        let removed = self.stream_map.write().await.remove(uuid).is_some();
        debug!("removed {} {} from cache", path.to_string_lossy(), uuid);
        removed
    }

    /// Returns a list of all available streams ready for playback sorted by on
    /// most recent date first. Note that even though they are available, the
    /// actual sources might be offline for what reason.
    ///
    /// # Arguments
    ///
    /// * `prefix` prefixes all urls contained in the [Stream] Vector with the
    ///   given argument
    ///
    /// # Return
    ///
    /// vector of registered streams
    pub async fn get_available_streams(&self, prefix: &'static str) -> Vec<Stream> {
        let mut map: Vec<Stream> = self
            .stream_map
            .read()
            .await
            .values()
            .cloned()
            .map(|s| prepend_prefix(s, prefix))
            .collect();

        map.sort_by(|a, b| b.date.cmp(&a.date));
        map
    }

    /// registers a new fixture
    pub async fn register(
        &self,
        description: String,
        sources: Vec<PathBuf>,
        date: DateTime<Utc>,
    ) -> Result<Uuid, RegisterError> {
        if sources.is_empty() {
            return Err(RegisterError::SourceArgumentEmpty);
        }

        let registration = MetaFile {
            uuid: Uuid::new_v4(),
            sources,
            description,
            date,
            live: Some(true),
        };

        let as_str = serde_yaml::to_string(&registration).map_err(RegisterError::ParseError)?;
        let name = format!("{}.stream", registration.uuid);

        let file_name = self.root.join(name);
        tokio::fs::write(&file_name, as_str.as_bytes())
            .await
            .map_err(RegisterError::IoError)?;

        info!("created {}", file_name.to_string_lossy(),);
        Ok(registration.uuid)
    }

    pub async fn get_segment<P>(&self, file: P) -> Result<Bytes>
    where
        P: Hash + AsRef<Path>,
    {
        let path = self.root.join(file.as_ref());
        // if let Some(buffer) = self.cache_controller.read().await.get() {
        //     return Ok(buffer.clone());
        // }

        // Ok(self
        //     .cache_controller
        //     .write()
        //     .await
        //     .insert(file.as_ref(), Arc::new(tokio::fs::read(path).await?))
        //     .clone())

        Ok(tokio::fs::read(path).await?.into())
    }
}

/// Adds a given url as prefix to the current base url. This base url is
fn prepend_prefix(mut stream: Stream, prefix: &'static str) -> Stream {
    for source in stream.sources.iter_mut() {
        let full = PathBuf::from(prefix).join(source.url.clone());
        source.url = full;
    }

    stream
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;

    fn assert_stream(mut a: Stream, mut b: Stream) {
        a.date = Utc::now();
        b.date = a.date;
        assert_eq!(a, b);
    }

    #[tokio::test]
    async fn test_scan() {
        let temp = TempDir::new("test").unwrap();
        let stream_store = LocalStreamStore::new(temp.path()).await;
        assert_eq!(0, stream_store.scan(temp.path()).await.unwrap().count());

        tokio::fs::File::create(temp.path().join("asdfa.bla"))
            .await
            .unwrap();

        assert_eq!(0, stream_store.scan(temp.path()).await.unwrap().count());
        assert_eq!(0, stream_store.stream_map.read().await.len());

        let registered = stream_store
            .register(
                "asdfas".to_string(),
                vec![PathBuf::from("test1.dash"), PathBuf::from("test1.m3u8")],
                Utc::now(),
            )
            .await
            .unwrap();
        let file_name = temp
            .path()
            .join(format!("{}.stream", registered.to_string()));
        assert_eq!(1, stream_store.scan(temp.path()).await.unwrap().count());

        tokio::fs::File::create(temp.path().join("test1.dash"))
            .await
            .unwrap();

        assert_eq!(
            0,
            stream_store
                .scan(&temp.path().join("test1.dash"))
                .await
                .unwrap()
                .count()
        );
    }

    #[tokio::test]
    async fn test_load_and_remove() {
        let temp = TempDir::new("test").unwrap();
        let stream_store = LocalStreamStore::new(temp.path()).await;

        let registered = stream_store
            .register(
                "test1".to_string(),
                vec![PathBuf::from("test1.dash"), PathBuf::from("test1.m3u8")],
                Utc::now(),
            )
            .await
            .unwrap();

        tokio::fs::File::create(temp.path().join("test1.dash"))
            .await
            .unwrap();

        tokio::fs::File::create(temp.path().join("blaat.dash"))
            .await
            .unwrap();

        LocalStreamStore::load(&stream_store.clone(), temp.path().to_path_buf())
            .await
            .unwrap();
        assert_stream(
            Stream {
                uuid: registered,
                description: "test1".to_string(),
                date: Utc::now(),
                sources: vec![
                    Source {
                        url: "test1.dash".into(),
                        typ: "application/dash+xml".to_string(),
                    },
                    Source {
                        url: "test1.m3u8".into(),
                        typ: "application/x-mpegURL".to_string(),
                    },
                ],
                live: Some(true),
            },
            stream_store.stream_map.read().await[&registered].clone(),
        );

        let uuid2 = stream_store
            .register(
                "12345".to_string(),
                vec![PathBuf::from("test2.dash"), PathBuf::from("test_3.m3u8")],
                Utc::now(),
            )
            .await
            .unwrap();

        LocalStreamStore::load(
            &stream_store.clone(),
            temp.path().join(format!("{}.stream", uuid2.to_string())),
        )
        .await
        .unwrap();
        assert_stream(
            Stream {
                uuid: uuid2,
                description: "12345".to_string(),
                sources: vec![
                    Source {
                        url: "test2.dash".into(),
                        typ: "application/dash+xml".to_string(),
                    },
                    Source {
                        url: "test_3.m3u8".into(),
                        typ: "application/x-mpegURL".to_string(),
                    },
                ],
                date: Utc::now(),
                live: Some(true),
            },
            stream_store.stream_map.read().await[&uuid2].clone(),
        );

        assert_stream(
            Stream {
                uuid: registered,
                description: "test1".to_string(),
                sources: vec![
                    Source {
                        url: "test1.dash".into(),
                        typ: "application/dash+xml".to_string(),
                    },
                    Source {
                        url: "test1.m3u8".into(),
                        typ: "application/x-mpegURL".to_string(),
                    },
                ],
                date: Utc::now(),
                live: Some(true),
            },
            stream_store.stream_map.read().await[&registered].clone(),
        );

        assert!(
            !stream_store
                .removed(stream_store.root.join("1234_test1.stream"))
                .await
        );

        let filename = format!("{}.stream", uuid2);
        stream_store.removed(stream_store.root.join(filename)).await;

        assert!(!stream_store.stream_map.read().await.contains_key(&uuid2));
    }

    #[tokio::test]
    async fn test_modification_of_stream_file() {
        let temp = TempDir::new("test").unwrap();
        let stream_store = LocalStreamStore::new(temp.path()).await;

        let registered = stream_store
            .register(
                "test1".to_string(),
                vec![PathBuf::from("test1.dash"), PathBuf::from("test1.m3u8")],
                Utc::now(),
            )
            .await
            .unwrap();
        stream_store.load(temp.path().to_path_buf()).await.unwrap();
        assert!(stream_store
            .stream_map
            .read()
            .await
            .contains_key(&registered));

        let filename = temp.path().join(format!("{}.stream", registered));
        let mut meta = stream_store.parse_file(&filename).unwrap().1;
        meta.description = "kees".to_string();
        let as_str = serde_yaml::to_string(&meta).unwrap();
        std::fs::write(filename, as_str).unwrap();
        stream_store.load(temp.path().to_path_buf()).await.unwrap();

        assert_eq!(
            "kees",
            &stream_store.stream_map.read().await[&registered].description
        );
    }
}
