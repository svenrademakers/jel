mod data_types;
mod file_watcher;
pub mod interface;

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs::DirEntry,
    ops::Deref,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        mpsc::{channel, Receiver},
        Arc,
    },
};

use self::{
    data_types::MetaFile,
    interface::{RegisterError, Stream, StreamId, StreamStore},
};
use async_trait::async_trait;
use futures_util::{pin_mut, Future, StreamExt};
use log::{debug, error, info, warn};
use notify::{DebouncedEvent, RecursiveMode, Watcher};
use tokio::{
    io::AsyncWriteExt,
    join,
    sync::{RwLock, RwLockReadGuard},
    task::JoinHandle,
};

/// Implementation of the streamstore trait where all streams reside on disk.
/// This object updates its internal bookkeeping when file changes happen. a new
/// stream is detected if a meta file, [StreamMetaFile], is detected in the
/// specified root folder.
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
    file_cache: RwLock<BTreeMap<PathBuf, StreamId>>,
    /// This watcher object is used to flag if init is called on the object.
    watcher_sender: Option<tokio::sync::mpsc::Sender<()>>,
}

impl LocalStreamStore {
    const FILTERED_EXTENSIONS: [&'static str; 3] = ["dash", "m3u8", "mpd"];

    pub async fn new(root: &Path) -> Arc<Self> {
        if !root.exists() {
            warn!("creating {}, does not exist", root.to_string_lossy());
            tokio::fs::create_dir_all(&root).await.unwrap();
        }

        Arc::new(LocalStreamStore {
            root: root.to_path_buf(),
            store: RwLock::new(BTreeMap::default()),
            file_cache: RwLock::new(BTreeMap::new()),
            watcher_sender: None,
        })
    }

    pub fn run(instance: &mut Arc<LocalStreamStore>) {
        let (kill_sender, kill_receiver) = tokio::sync::mpsc::channel(8);

        let value =
            Arc::get_mut(instance).expect("LocalStreamStore cannot be shared before run call");
        value.watcher_sender = Some(kill_sender);

        // spawn loading task
        tokio::spawn(LocalStreamStore::load(
            instance.clone(),
            instance.root.to_path_buf(),
        ));

        // spawn file watcher task
        Self::watch_for_changes(instance.clone(), kill_receiver);
    }

    async fn load(self: Arc<LocalStreamStore>, path: PathBuf) -> Option<()> {
        let stream_iter = tokio_stream::iter(self.scan(&path).await.ok()?)
            .filter_map(|f| MetaFile::into_metadata(f, &self.root));
        pin_mut!(stream_iter);
        let mut recordings_map = self.store.write().await;
        while let Some((key, stream)) = stream_iter.next().await {
            if let Some(value) = recordings_map.get(&key) {
                error!("skipping {:?}{:?}, duplicate of {:?}", key, stream, value);
                continue;
            }
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
            debug!("scanning {}", relative.to_string_lossy());

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
                    Some(ext) => {
                        if LocalStreamStore::FILTERED_EXTENSIONS.contains(&ext) {
                            loose_files.insert(relative, StreamId::None);
                        }
                    }
                    _ => debug!("{} not of interest", path.to_string_lossy()),
                };
            } else {
                error!("error opening {:?}", path);
            }
        };

        debug!("scanning: {:?}", &path);
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

        self.file_cache.write().await.extend(loose_files);

        info!(
            "found {} stream(s) in {}",
            found.len(),
            path.to_string_lossy()
        );

        info!("cache size: {} entries", self.file_cache.read().await.len());
        Ok(found.into_iter())
    }

    pub async fn removed(&self, path: PathBuf) {
        let mut stream_store = self.store.write().await;
        let mut file_cache = self.file_cache.write().await;
        let path = path.strip_prefix(&self.root).unwrap();

        match file_cache.get(path) {
            Some(id) if path.extension().unwrap_or_default() == "stream" => {
                stream_store.remove(id);
            }
            None => return,
            _ => (),
        }
        file_cache.remove(path);
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

#[async_trait]
impl StreamStore for LocalStreamStore {
    async fn get_fixtures(&self, prefix: &'static str) -> BTreeMap<u32, Stream> {
        let map = self.store.read().await.clone();
        prepend_prefix(map.into_iter(), prefix)
            .filter_map(|(id, stream)| match id {
                StreamId::FootballAPI(id) => Some((id, stream)),
                StreamId::Untagged(_) => None,
                StreamId::None => None,
            })
            .collect()
    }

    /// registers a new fixture
    async fn register(
        &self,
        id: StreamId,
        sources: Vec<PathBuf>,
        live: bool,
        title: Option<PathBuf>,
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
            live: Some(live),
            title,
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

    async fn get_untagged_sources(&self) -> Vec<PathBuf> {
        self.file_cache
            .read()
            .await
            .iter()
            .filter_map(|(k, v)| {
                if &StreamId::None == v {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<PathBuf>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        logger::init_log,
        middleware::interface::{Source, StreamingType},
    };
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
        let temp = TempDir::new("test").unwrap();
        let stream_store = LocalStreamStore::new(temp.path()).await;
        assert_eq!(0, stream_store.scan(temp.path()).await.unwrap().count());

        tokio::fs::File::create(temp.path().join("asdfa.bla"))
            .await
            .unwrap();
        assert_eq!(0, stream_store.scan(temp.path()).await.unwrap().count());
        assert_eq!(0, stream_store.file_cache.read().await.len());

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
            let cache = stream_store.file_cache.read().await;

            assert_eq!(
                Some(&StreamId::FootballAPI(1234)),
                cache.get(&PathBuf::from("1234_test1.stream"))
            );
            assert_eq!(
                Some(&StreamId::FootballAPI(1234)),
                cache.get(&PathBuf::from("test1.dash"))
            );
        }
        let cache = stream_store.get_untagged_sources().await;
        assert_eq!(vec![PathBuf::from("nonsense.dash").as_path()], cache);
    }

    #[tokio::test]
    async fn test_load_and_remove() {
        init_log(log::Level::Debug);
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

        LocalStreamStore::load(stream_store.clone(), temp.path().to_path_buf()).await;
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

        LocalStreamStore::load(stream_store.clone(), temp.path().join("2_test2.stream")).await;

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
        let cache = stream_store.file_cache.read().await;
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
            &stream_store.get_untagged_sources().await.as_slice()
        );

        stream_store
            .removed(stream_store.root.join("blaat.dash"))
            .await;
        assert!(stream_store.get_untagged_sources().await.is_empty());
    }
}
