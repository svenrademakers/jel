pub mod data_types;
mod file_watcher;

use self::data_types::*;
use super::cache_map::CacheController;
use chrono::{DateTime, Utc};
use futures_util::{pin_mut, StreamExt};
use log::{debug, error, info, trace, warn};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    hash::Hash,
    io::{self, Write},
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTimeError,
};
use tokio::sync::RwLock;

/// Implements functionality to get video streams that are persisted on a filesystem.
/// This object updates its internal bookkeeping when file changes happen. a new
/// stream is detected if a meta file, [StreamMetaFile], is detected in the
/// specified root folder.
///
/// # About Streams
///
/// Currently 2 types of streams exist:
/// 1. Football fixture related. These streams correspond to an actual football
///    fixture.
/// 2. Untagged. These are streams that do not direct relate to an actual
///    fixture, but can be any content.
///
/// Football streams have the benefit in that they can be correlated with other
/// football information and systems. The actual key is dictated by an external
/// trait, see [super::FootballInfo].
///
/// you can assume the following about stream data:
/// * [StreamId] defines the unique key to index an stream
/// * [StreamId::FootballAPI] should reference a valid `fixture_id`
/// * a [Stream] can contain multiple sources in multiple formats. This is to
///   offer viewers compatibility and the choice to throttle different
///   qualities. All sources show the same content!
///
/// Implementation of the streamstore trait where all streams reside on disk.

pub struct LocalStreamStore {
    /// Directory which all stream files will be written to. All paths used in
    /// [StreamStoreImpl] are relative compared to the root directory
    root: PathBuf,
    /// Map that contains the index of found streams. This is the single source
    /// of truth.
    store: RwLock<BTreeMap<StreamId, Stream>>,
    /// A cache that reflects the files that are present inside the root. files
    /// that have the id [StreamId::None] are not associated with any stream.
    /// When the files on disk change this cache should be updated as well.
    /// Equally when a file gets associated with a stream the streamId should be
    /// updated to reflect this.
    fs_cache: RwLock<BTreeMap<PathBuf, StreamId>>,
    cache_controller: RwLock<CacheController<Path, Arc<Vec<u8>>, 128>>,
    /// This watcher object is used to flag if init is called on the object.
    watcher_sender: Option<tokio::sync::oneshot::Sender<()>>,
}

impl LocalStreamStore {
    const FILTERED_EXTENSIONS: [&'static str; 3] = ["dash", "m3u8", "mpd"];
    pub async fn new(root: &Path) -> Arc<LocalStreamStore> {
        if !root.exists() {
            warn!("creating {}, does not exist", root.to_string_lossy());
            tokio::fs::create_dir_all(&root).await.unwrap();
        }

        //let cache_storage = std::iter::repeat_with(|| V::default()).take(N).collect();

        Arc::new(LocalStreamStore {
            root: root.to_path_buf(),
            store: RwLock::new(BTreeMap::default()),
            fs_cache: RwLock::new(BTreeMap::new()),
            cache_controller: RwLock::new(CacheController::new()),
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
                .await;
        });

        // spawn file watcher task
        Self::watch_for_changes(instance.clone(), kill_receiver);
    }

    async fn load(&self, path: PathBuf) -> Option<()> {
        let stream_iter = tokio_stream::iter(self.scan(&path).await.ok()?)
            .filter_map(|f| MetaFile::into_metadata(f, &self.root));
        pin_mut!(stream_iter);
        let mut recordings_map = self.store.write().await;
        while let Some((key, stream)) = stream_iter.next().await {
            recordings_map.insert(key, stream);
        }

        Some(())
    }

    /// scans for .stream files in a given path none recursively. If path is not a
    /// sub directory of root, None is returned.
    async fn scan(&self, path: &Path) -> std::io::Result<impl Iterator<Item = MetaFile>> {
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

        let mut found: Vec<MetaFile> = Vec::new();
        let mut loose_files = BTreeMap::new();

        let mut load_file = |path: &Path| {
            let relative = path
                .strip_prefix(&self.root)
                .expect("root tested on the start of the function")
                .to_path_buf();
            trace!("scanning {}", relative.to_string_lossy());

            if let Ok(file) = std::fs::File::open(path) {
                match path.extension().and_then(OsStr::to_str) {
                    Some("stream") => {
                        match serde_yaml::from_reader::<std::fs::File, MetaFile>(file) {
                            Ok(stream) => {
                                loose_files.insert(relative, stream.id.clone());
                                found.push(stream);
                            }
                            Err(e) => {
                                error!("could not parse {} {}", path.to_string_lossy(), e);
                            }
                        }
                    }
                    _ => debug!("{} not of interest", path.to_string_lossy()),
                };
            } else {
                error!("error opening {:?}", path);
            }
        };

        trace!("scanning: {:?}", &path);
        let md = tokio::fs::metadata(path).await?;
        if md.is_file() {
            load_file(path);
        } else {
            let mut dir_entry = tokio::fs::read_dir(path).await?;
            while let Ok(Some(entry)) = dir_entry.next_entry().await {
                load_file(&entry.path());
            }
        }

        for meta_file in &found {
            let iter = meta_file
                .filenames
                .iter()
                .cloned()
                .map(|f| (f, meta_file.id.clone()));
            loose_files.extend(iter);
        }

        self.fs_cache.write().await.extend(loose_files);

        debug!(
            "found {} stream(s) in {}",
            found.len(),
            path.to_string_lossy()
        );
        debug!("cache size: {} entries", self.fs_cache.read().await.len());
        Ok(found.into_iter())
    }

    pub async fn removed(&self, path: PathBuf) {
        let mut stream_store = self.store.write().await;
        let mut file_cache = self.fs_cache.write().await;
        let path = path.strip_prefix(&self.root).unwrap();

        match file_cache.get(path).cloned() {
            Some(id) if path.extension().unwrap_or_default() == "stream" => {
                if let Some(stream) = stream_store.get(&id) {
                    for source in &stream.sources {
                        file_cache.remove(&source.url);
                    }
                    stream_store.remove(&id);
                }
            }
            None => return,
            _ => (),
        }
        file_cache.remove(path);
        debug!("removed {} from cache", path.to_string_lossy());
    }

    pub async fn get_all(&self, prefix: &'static str) -> BTreeMap<u32, Stream> {
        let map = self.store.read().await.clone();
        prepend_prefix(map.into_iter(), prefix)
            .map(|(id, stream)| (id.get_raw_key().unwrap(), stream))
            .collect()
    }

    pub async fn get_fixtures(&self, prefix: &'static str) -> BTreeMap<u32, Stream> {
        let map = self.store.read().await.clone();
        prepend_prefix(map.into_iter(), prefix)
            .filter_map(|(id, stream)| match id {
                StreamId::FootballAPI(id) => Some((id, stream)),
                _ => None,
            })
            .collect()
    }

    /// registers a new fixture
    pub async fn register(
        &self,
        id: StreamId,
        sources: Vec<PathBuf>,
        title: String,
        date: DateTime<Utc>,
    ) -> Result<(), RegisterError> {
        if sources.is_empty() {
            return Err(RegisterError::SourceArgumentEmpty);
        }

        if let Some(entry) = self.store.read().await.get(&id) {
            return Err(RegisterError::IdAlreadyRegisteredTo(entry.clone()));
        }

        let registration = MetaFile {
            id,
            filenames: sources,
            title,
            date,
        };

        let as_str = serde_yaml::to_string(&registration).map_err(RegisterError::ParseError)?;
        let name = format!(
            "{}_{}.stream",
            registration.id.get_raw_key().unwrap_or_default(),
            registration
                .filenames
                .first()
                .unwrap()
                .file_stem()
                .unwrap()
                .to_string_lossy()
        );

        let file_name = self.root.join(name);
        tokio::fs::write(&file_name, as_str.as_bytes())
            .await
            .unwrap();

        info!(
            "created {} with size of {} bytes",
            file_name.to_string_lossy(),
            as_str.len()
        );
        Ok(())
    }

    pub async fn get_untagged_sources(&self, prefix: &'static str) -> Vec<PathBuf> {
        self.fs_cache
            .read()
            .await
            .iter()
            .filter_map(|(k, v)| {
                if &StreamId::None == v {
                    Some(PathBuf::from(prefix).join(k))
                } else {
                    None
                }
            })
            .collect::<Vec<PathBuf>>()
    }

    pub async fn get_source<P>(&self, file: P) -> std::io::Result<Arc<Vec<u8>>>
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
fn prepend_prefix<T: Iterator<Item = (StreamId, Stream)>>(
    iter: T,
    prefix: &'static str,
) -> impl Iterator<Item = (StreamId, Stream)> {
    iter.map(move |(k, mut v)| {
        for source in v.sources.iter_mut() {
            let full = PathBuf::from(prefix).join(source.url.clone());
            source.url = full;
        }

        (k, v)
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
