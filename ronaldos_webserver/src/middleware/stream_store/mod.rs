pub mod data_types;

use self::data_types::*;
use super::cache_map::CacheMap;
use anyhow::{bail, ensure, Context, Result};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use log::{debug, error, info, trace, warn};
use notify::{RecommendedWatcher, recommended_watcher, RecursiveMode, EventKind, Watcher};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    hash::Hash,
    path::{Path, PathBuf}, sync::Arc,
};
use tokio::sync::{RwLock, mpsc::{Receiver, channel}};
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
    file_cache: RwLock<CacheMap<Path, Bytes, 128>>,
    /// This watcher object is used to exit the watcher task.
    file_watcher: Option<RecommendedWatcher>,
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
            file_cache: RwLock::new(CacheMap::new()),
            file_watcher: None,
        })
    }

    pub fn run(instance: &mut Arc<LocalStreamStore>) {
        let mut receiver = Self::start_filewatcher(instance);

        // spawn loading task
        let loading_instance = instance.clone();
        tokio::spawn(async move {
            loading_instance
                .load(&[loading_instance.root.to_path_buf(); 1])
                .await
                .unwrap();
        });

        // spawn file watcher task
        let watch_instance = instance.clone();
        tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                watch_instance.handle_debounce_event(event).await;
            }
        });
    }

    fn start_filewatcher(instance: &mut Arc<LocalStreamStore>)-> Receiver<notify::Event>{
        let (sender, receiver) = channel(32);
        let handle_notify_receiver = move |res| {
            let event = match res {
                Ok(event) => event,
                Err(e) => { error!("cannot handle notify event because {}", e);
                    return;
                },
            };
            trace!("received {:?}", &event);
            if let Err(e) = sender.blocking_send(event){
                warn!("channel failure to filewatcher: {}", e);
            }
        };

        let mut watcher = recommended_watcher(handle_notify_receiver).unwrap();
        watcher.watch(&instance.root, RecursiveMode::NonRecursive).unwrap();
        let mut_instance =
            Arc::get_mut(instance).expect("LocalStreamStore is not allowed to be shared before run call");
        mut_instance.file_watcher = Some(watcher);
        receiver
    }

    async fn handle_debounce_event(
        &self,
        event: notify::event::Event,
        ) {
        let paths = event.paths;
        let result: Result<()> = match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => self.load(&paths).await,
            EventKind::Remove(_) => {
                self.removed(&paths).await.then(|| ()).context("nothing removed")
            }
            //          DebouncedEvent::Rename(rem, add) => {
            //              stream_store.removed(rem).await;
            //              stream_store.load(add).await
            //          }
            _ => Ok(()),
        };

        if let Err(e) = result {
            warn!("failed handling event: {}", e);
        }
    }

    /// Load a .stream meta file from disk. path can be a directory or a file.
    /// note that recursive scanning is disabled. see [LocalStreamStore::scan]
    async fn load(&self, paths: &[PathBuf]) -> Result<()> {
        let mut lookup = Vec::new();
        let mut new_meta_files = Vec::new();
        for path in paths {
            new_meta_files.extend(self.scan(&path).await?.map(|(p, meta)| {
                lookup.push((p, meta.uuid));
                (meta.uuid, meta.into())
            }));
        }
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

    pub async fn removed(&self, paths: &[PathBuf]) -> bool {
        let mut removed_count = 0;
        let files = paths.iter().filter(|f| f.extension() == Some(OsStr::new("stream")));

        let lookup = self.uuid_lookup.read().await;
        for file in files {
            let uuid = match lookup.get(file) {
                Some(val) => val,
                None => continue,
            };

            if let Some(_) = self.stream_map.write().await.remove(uuid){
                debug!("removed {} {} from cache", file.to_string_lossy(), uuid);
                removed_count += 1;
            }
        }
        removed_count > 0
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
    pub async fn get_available_streams(&self, prefix: &str) -> Vec<Stream> {
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
        let mut cache = self.file_cache.write().await;
        let path = self.root.join(file.as_ref());
        if let Some(buffer) = cache.get(&path) {
            return Ok(buffer.clone());
        }

        Ok(cache
            .insert(file.as_ref(), tokio::fs::read(path).await?.into())
            .clone())
    }
}

/// Adds a given url as prefix to the current base url. This base url is
fn prepend_prefix(mut stream: Stream, prefix: &str) -> Stream {
    for source in stream
        .sources
        .iter_mut()
        .filter(|s| !s.url.starts_with("http:") && !s.url.starts_with("https:"))
    {
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

        let _ = stream_store
            .register(
                "asdfas".to_string(),
                vec![PathBuf::from("test1.dash"), PathBuf::from("test1.m3u8")],
                Utc::now(),
            )
            .await
            .unwrap();

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
