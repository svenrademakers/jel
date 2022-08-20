pub mod data_types;
mod file_watcher;

use self::data_types::*;
use super::cache_map::CacheController;
use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use log::{debug, error, info, trace, warn};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    hash::Hash,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::RwLock;

/// Provides video streams that are persisted on the filesystem. Even given they
/// are written real-time. At the moment there is support for the followin
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
/// accomodate most bandwith capabilities of clients watching theses streams. To
/// accomodate for this as best as possible we have 3 caches for each level, so
/// we keep the amount of memory allocations at a minimum
pub struct LocalStreamStore {
    /// Directory which all stream files will be written to. All paths used in
    /// [StreamStoreImpl] are relative compared to the root directory
    root: PathBuf,
    /// Map that contains the index of found streams. This is the single source
    /// of truth.
    stream_map: RwLock<BTreeMap<String, Stream>>,
    /// 3 way cache, caching streams optimized for the 3 different bitrate levels.
    file_cache: RwLock<CacheController<Path, Arc<Vec<u8>>, 128>>,
    /// This watcher object is used to exit the watcher task.
    watcher_sender: Option<tokio::sync::oneshot::Sender<()>>,
}

impl LocalStreamStore {
    const FILTERED_EXTENSIONS: [&'static str; 3] = ["dash", "m3u8", "mpd"];
    pub async fn new(root: &Path) -> Arc<LocalStreamStore> {
        if !root.exists() {
            warn!("creating {}, does not exist", root.to_string_lossy());
            tokio::fs::create_dir_all(&root).await.unwrap();
        }

        Arc::new(LocalStreamStore {
            root: root.to_path_buf(),
            stream_map: RwLock::new(BTreeMap::default()),
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
    async fn load(&self, path: PathBuf) -> std::io::Result<()> {
        let new_meta_files = tokio_stream::iter(self.scan(&path).await?)
            .map(|(p, meta)| (p, meta.into()))
            .collect::<Vec<(String, Stream)>>()
            .await;

        self.stream_map.write().await.extend(new_meta_files);
        Ok(())
    }

    /// Scans for .stream files in a given path none recursively. If path is not a
    /// sub directory of root, None is returned.
    async fn scan(&self, path: &Path) -> std::io::Result<impl Iterator<Item = (String, MetaFile)>> {
        if !path.starts_with(&self.root) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "{} is not in {}",
                    path.to_string_lossy(),
                    self.root.to_string_lossy()
                ),
            ));
        }
        trace!("scanning: {:?}", &path);

        let mut found = Vec::new();
        let mut push_found = |path| {
            if let Some(tuple) = self.parse_file(path) {
                found.push(tuple);
            }
        };

        let md = tokio::fs::metadata(path).await?;
        if md.is_file() {
            push_found(path);
        } else {
            let mut dir_entry = tokio::fs::read_dir(path).await?;
            while let Ok(Some(entry)) = dir_entry.next_entry().await {
                push_found(path);
            }
        }

        debug!(
            "found {} stream(s) in {}",
            found.len(),
            path.to_string_lossy()
        );

        Ok(found.into_iter())
    }

    fn parse_file(&self, path: &Path) -> Option<(String, MetaFile)> {
        if path.ends_with("stream") {
            return None;
        }

        let relative = path
            .strip_prefix(&self.root)
            .expect("root tested on the start of the function")
            .to_path_buf();
        trace!("scanning {}", relative.to_string_lossy());

        let file;
        match std::fs::File::open(path) {
            Ok(f) => file = f,
            Err(e) => {
                error!("error opening {:?}: {}", path, e);
                return None;
            }
        }
        match serde_yaml::from_reader::<std::fs::File, MetaFile>(file) {
            Ok(stream) => {
                let stem = String::from(
                    path.file_stem()
                        .expect("not curropted file_stem")
                        .to_str()
                        .unwrap(),
                );
                Some((stem, stream.into()))
            }
            Err(e) => {
                error!("could not parse {} {}", path.to_string_lossy(), e);
                None
            }
        }
    }

    pub async fn removed(&self, path: PathBuf) -> bool {
        if path.extension() != Some(OsStr::new("stream")) {
            return false;
        }

        let key = path
            .file_stem()
            .expect("path should reside in the root directory")
            .to_str()
            .unwrap();
        let removed = self.stream_map.write().await.remove(key).is_some();
        debug!("removed {} from cache", path.to_string_lossy());
        removed
    }

    pub async fn get_available_streams(&self, prefix: &'static str) -> Vec<Stream> {
        let map = self.stream_map.read().await;
        prepend_prefix(map.values().cloned(), prefix).collect()
    }

    /// registers a new fixture
    pub async fn register(
        &self,
        name: String,
        description: String,
        sources: Vec<PathBuf>,
        date: DateTime<Utc>,
    ) -> Result<(), RegisterError> {
        if sources.is_empty() {
            return Err(RegisterError::SourceArgumentEmpty);
        }

        if let Some(entry) = self.stream_map.read().await.get(&name) {
            return Err(RegisterError::IdAlreadyRegisteredTo(entry.clone()));
        }

        let registration = MetaFile {
            filenames: sources,
            description,
            date,
        };

        let as_str = serde_yaml::to_string(&registration).map_err(RegisterError::ParseError)?;
        let name = format!("{}.stream", name);

        let file_name = self.root.join(name);
        tokio::fs::write(&file_name, as_str.as_bytes())
            .await
            .map_err(|e| RegisterError::IoError(e))?;

        info!("created {}", file_name.to_string_lossy(),);
        Ok(())
    }

    pub async fn get_segment<P>(&self, file: P) -> std::io::Result<Arc<Vec<u8>>>
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

        Ok(Arc::new(tokio::fs::read(path).await?))
    }
}

/// Adds a given url as prefix to the current base url. This base url is
fn prepend_prefix<T: Iterator<Item = Stream>>(
    iter: T,
    prefix: &'static str,
) -> impl Iterator<Item = Stream> {
    iter.map(move |mut stream| {
        for source in stream.sources.iter_mut() {
            let full = PathBuf::from(prefix).join(source.url.clone());
            source.url = full;
        }

        stream
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::logger::init_log;
    use tempdir::TempDir;

    fn assert_stream(mut a: Stream, mut b: Stream) {
        a.sources
            .iter_mut()
            .for_each(|f| f.created = std::time::UNIX_EPOCH.into());
        b.sources
            .iter_mut()
            .for_each(|f| f.created = std::time::UNIX_EPOCH.into());
        assert_eq!(a, b);
    }

    #[tokio::test]
    async fn test_scan() {
        init_log(log::Level::Debug);

        let temp = TempDir::new("test").unwrap();
        let stream_store = LocalStreamStore::new(temp.path()).await;
        assert_eq!(0, stream_store.scan(temp.path()).await.unwrap().count());

        tokio::fs::File::create(temp.path().join("asdfa.bla"))
            .await
            .unwrap();
        assert_eq!(0, stream_store.scan(temp.path()).await.unwrap().count());
        assert_eq!(0, stream_store.fs_cache.read().await.len());

        stream_store
            .register(
                StreamId::FootballAPI(1234),
                vec![PathBuf::from("test1.dash"), PathBuf::from("test1.m3u8")],
                false,
                None,
            )
            .await
            .unwrap();
        assert_eq!(1, stream_store.scan(temp.path()).await.unwrap().count());

        tokio::fs::File::create(temp.path().join("test1.dash"))
            .await
            .unwrap();
        tokio::fs::File::create(temp.path().join("nonsense.dash"))
            .await
            .unwrap();

        assert_eq!(1, stream_store.scan(temp.path()).await.unwrap().count());

        {
            let cache = stream_store.fs_cache.read().await;

            assert_eq!(
                Some(&StreamId::FootballAPI(1234)),
                cache.get(&PathBuf::from("1234_test1.stream"))
            );
            assert_eq!(
                Some(&StreamId::FootballAPI(1234)),
                cache.get(&PathBuf::from("test1.dash"))
            );
        }
        let cache = stream_store.get_untagged_sources("").await;
        assert_eq!(vec![PathBuf::from("nonsense.dash").as_path()], cache);
    }

    #[tokio::test]
    async fn test_load_and_remove() {
        let temp = TempDir::new("test").unwrap();
        let stream_store = LocalStreamStore::new(temp.path()).await;

        stream_store
            .register(
                StreamId::FootballAPI(1234),
                vec![PathBuf::from("test1.dash"), PathBuf::from("test1.m3u8")],
                false,
                None,
            )
            .await
            .unwrap();

        tokio::fs::File::create(temp.path().join("test1.dash"))
            .await
            .unwrap();

        tokio::fs::File::create(temp.path().join("blaat.dash"))
            .await
            .unwrap();

        LocalStreamStore::load(&stream_store.clone(), temp.path().to_path_buf()).await;
        assert_stream(
            Stream {
                sources: vec![
                    Source {
                        url: "test1.dash".into(),
                        typ: StreamingType::DASH,
                        created: std::time::UNIX_EPOCH.into(),
                    },
                    Source {
                        url: "test1.m3u8".into(),
                        typ: StreamingType::HLS,
                        created: std::time::UNIX_EPOCH.into(),
                    },
                ],
                live: false,
            },
            stream_store.store.read().await[&StreamId::FootballAPI(1234)].clone(),
        );

        stream_store
            .register(
                StreamId::Untagged(2),
                vec![PathBuf::from("test2.dash"), PathBuf::from("test_3.m3u8")],
                true,
                None,
            )
            .await
            .unwrap();

        LocalStreamStore::load(&stream_store.clone(), temp.path().join("2_test2.stream")).await;

        assert_stream(
            Stream {
                sources: vec![
                    Source {
                        url: "test2.dash".into(),
                        typ: StreamingType::DASH,
                        created: std::time::UNIX_EPOCH.into(),
                    },
                    Source {
                        url: "test_3.m3u8".into(),
                        typ: StreamingType::HLS,
                        created: std::time::UNIX_EPOCH.into(),
                    },
                ],
                live: true,
            },
            stream_store.store.read().await[&StreamId::Untagged(2)].clone(),
        );

        assert_stream(
            Stream {
                sources: vec![
                    Source {
                        url: "test1.dash".into(),
                        typ: StreamingType::DASH,
                        created: std::time::UNIX_EPOCH.into(),
                    },
                    Source {
                        url: "test1.m3u8".into(),
                        typ: StreamingType::HLS,
                        created: std::time::UNIX_EPOCH.into(),
                    },
                ],
                live: false,
            },
            stream_store.store.read().await[&StreamId::FootballAPI(1234)].clone(),
        );
        let cache = stream_store.fs_cache.read().await;
        assert_eq!(
            StreamId::FootballAPI(1234),
            cache[&PathBuf::from("1234_test1.stream")]
        );
        assert_eq!(
            StreamId::FootballAPI(1234),
            cache[&PathBuf::from("test1.dash")]
        );
        assert_eq!(
            StreamId::FootballAPI(1234),
            cache[&PathBuf::from("test1.m3u8")]
        );
        assert_eq!(
            StreamId::Untagged(2),
            cache[&PathBuf::from("2_test2.stream")]
        );
        assert_eq!(StreamId::Untagged(2), cache[&PathBuf::from("test2.dash")]);
        assert_eq!(StreamId::Untagged(2), cache[&PathBuf::from("test_3.m3u8")]);
        assert_eq!(StreamId::None, cache[&PathBuf::from("blaat.dash")]);
        assert_eq!(7, cache.len());
        drop(cache);

        assert_eq!(
            &[PathBuf::from("blaat.dash")],
            &stream_store.get_untagged_sources("").await.as_slice()
        );

        stream_store
            .removed(stream_store.root.join("blaat.dash"))
            .await;
        assert!(stream_store.get_untagged_sources("").await.is_empty());

        stream_store
            .removed(stream_store.root.join("1234_test1.stream"))
            .await;

        assert!(!stream_store
            .store
            .read()
            .await
            .contains_key(&StreamId::FootballAPI(1234)));

        let cache = stream_store.fs_cache.read().await;
        assert!(!cache.contains_key(&PathBuf::from("test1.dash")));
        assert!(!cache.contains_key(&PathBuf::from("test1.m3u8")));
    }
}
