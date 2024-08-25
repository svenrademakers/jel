pub mod data_types;

use self::data_types::*;
use anyhow::{bail, ensure, Context, Result};
use chrono::{DateTime, Utc};
use hashbrown::HashMap;
use notify::{Config, Event, EventKind, PollWatcher, RecursiveMode, Watcher};
use std::{
    ffi::OsStr,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{mpsc::channel, mpsc::Receiver, RwLock};
use tracing::{debug, error, info, instrument, trace, warn};
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
    request_base: PathBuf,
    /// Map that contains the index of found streams. This is the single source
    /// of truth.
    stream_map: HashMap<Uuid, Stream>,
    uuid_lookup: HashMap<PathBuf, Uuid>,
    /// This watcher object is used to exit the watcher task.
    file_watcher: Option<PollWatcher>,
}

impl LocalStreamStore {
    pub fn new(root: PathBuf, request_base: PathBuf) -> LocalStreamStore {
        if !root.exists() {
            info!("creating {}, as it does not exist", root.to_string_lossy());
            std::fs::create_dir_all(&root).unwrap();
        }

        LocalStreamStore {
            root,
            request_base,
            stream_map: HashMap::default(),
            uuid_lookup: HashMap::default(),
            file_watcher: None,
        }
    }

    pub async fn run(instance: &Arc<RwLock<LocalStreamStore>>) {
        // spawn loading task
        let loading_instance = instance.clone();
        tokio::spawn(async move {
            let mut unlocked = loading_instance.write().await;
            let root_path = unlocked.root.clone();
            unlocked.load(&[root_path; 1]).unwrap();
        });

        // spawn file watcher task
        let mut receiver = instance.write().await.start_filewatcher();
        let watch_instance = instance.clone();
        tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                watch_instance
                    .write()
                    .await
                    .handle_debounce_event(event)
                    .await;
            }
        });
    }

    fn start_filewatcher(&mut self) -> Receiver<Event> {
        let (sender, receiver) = channel(32);
        let handle_notify_receiver = move |res| {
            let event = match res {
                Ok(event) => event,
                Err(e) => {
                    error!("cannot handle notify event because {}", e);
                    return;
                }
            };
            trace!("received {:?}", &event);

            if let Err(e) = sender.blocking_send(event) {
                warn!("channel failure to filewatcher: {}", e);
            }
        };

        let poll_config = Config::default().with_poll_interval(Duration::from_secs(5));
        let mut watcher = PollWatcher::new(handle_notify_receiver, poll_config).unwrap();
        watcher
            .watch(&self.root, RecursiveMode::NonRecursive)
            .unwrap();

        self.file_watcher = Some(watcher);
        receiver
    }

    async fn handle_debounce_event(&mut self, event: notify::event::Event) {
        let paths = event.paths;
        let result: Result<()> = match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => self.load(&paths),
            EventKind::Remove(_) => self
                .removed(&paths)
                .await
                .then_some(())
                .context("nothing removed"),
            _ => Ok(()),
        };

        if let Err(e) = result {
            warn!("failed handling event: {}", e);
        }
    }

    /// Load a .stream meta file from disk. path can be a directory or a file.
    /// note that recursive scanning is disabled. see [LocalStreamStore::scan]
    fn load(&mut self, paths: &[PathBuf]) -> Result<()> {
        let mut lookup = Vec::new();
        let mut new_meta_files = Vec::new();
        for path in paths {
            new_meta_files.extend(self.scan(path)?.map(|(p, meta)| {
                lookup.push((p, meta.uuid));
                (meta.uuid, meta.into())
            }));
        }

        self.uuid_lookup.extend(lookup);
        self.stream_map.extend(new_meta_files);
        Ok(())
    }

    /// Scans for .stream files in a given path none recursively. If path is not a
    /// sub directory of root, None is returned.
    fn scan(&self, path: &Path) -> Result<impl Iterator<Item = (PathBuf, MetaFile)>> {
        ensure!(
            path.starts_with(&self.root),
            format!(
                "{} is not in {}",
                path.to_string_lossy(),
                self.root.to_string_lossy()
            )
        );

        trace!("scanning: {:?}", &path);
        let mut found = Vec::with_capacity(512);
        let mut push_found = |path: &Path| {
            self.parse_file(path)
                .map_or_else(|e| warn!("{:#}", e), |tuple| found.push(tuple))
        };

        let meta_data = fs::metadata(path)
            .with_context(|| format!("failed to get metadata for {}", path.to_string_lossy()))?;

        if meta_data.is_file() {
            push_found(path);
        } else {
            let mut dir_entry = fs::read_dir(path)
                .with_context(|| format!("failed to read dir {}", path.to_string_lossy()))?;
            while let Some(Ok(entry)) = dir_entry.next() {
                push_found(&entry.path());
            }
        }

        debug!(
            "found/updated {} stream(s) in {}",
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

        let mut stream = serde_yaml::from_reader::<std::fs::File, MetaFile>(file)
            .with_context(|| format!("could not parse {}", path.to_string_lossy()))?;
        self.patch_sources(&mut stream);

        Ok((path.to_path_buf(), stream))
    }

    /// This function converts the actual paths on disk to request urls. This prevents us from
    /// having to convert sources during a given request.
    fn patch_sources(&self, stream: &mut MetaFile) {
        for source in stream.sources.iter_mut().filter(|s| !s.starts_with("http")) {
            let full = self.request_base.join(&source);
            *source = full;
        }
    }

    pub async fn removed(&mut self, paths: &[PathBuf]) -> bool {
        let mut removed_count = 0;
        let files = paths
            .iter()
            .filter(|f| f.extension() == Some(OsStr::new(STREAM_EXT)));

        for file in files {
            let Some(uuid) = self.uuid_lookup.remove(file) else {
                continue;
            };

            if self.stream_map.remove(&uuid).is_some() {
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
    /// # Return
    ///
    /// vector of registered streams
    pub fn get_available_streams(&self) -> impl Iterator<Item = &Stream> {
        self.stream_map.values()
    }

    #[instrument(skip(self, writer))]
    pub fn get_segment(&self, file: &Path, mut writer: impl Write) -> io::Result<()> {
        let path = self.root.join(file);
        debug!("reading segment {}", path.to_string_lossy());
        let mut f = fs::OpenOptions::new().read(true).open(&path)?;
        std::io::copy(&mut f, &mut writer)?;
        Ok(())
    }

    /// registers a new fixture
    pub async fn register(
        &self,
        description: String,
        sources: Vec<PathBuf>,
        date: DateTime<Utc>,
        fixture_id: Option<u64>,
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
            fixture_id,
        };

        let name = format!("{}.{}", registration.uuid, STREAM_EXT);
        let file_name = self.root.join(name);

        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&file_name)?;
        serde_yaml::to_writer(file, &registration)?;

        info!("created {}", file_name.to_string_lossy());
        Ok(registration.uuid)
    }
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
        assert_eq!(0, stream_store.scan(temp.path()).unwrap().count());

        tokio::fs::File::create(temp.path().join("asdfa.bla"))
            .await
            .unwrap();

        assert_eq!(0, stream_store.scan(temp.path()).unwrap().count());
        assert_eq!(0, stream_store.stream_map.read().await.len());

        let _ = stream_store
            .register(
                "asdfas".to_string(),
                vec![PathBuf::from("test1.dash"), PathBuf::from("test1.m3u8")],
                Utc::now(),
            )
            .await
            .unwrap();

        assert_eq!(1, stream_store.scan(temp.path()).unwrap().count());

        tokio::fs::File::create(temp.path().join("test1.dash"))
            .await
            .unwrap();

        assert_eq!(
            0,
            stream_store
                .scan(&temp.path().join("test1.dash"))
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
