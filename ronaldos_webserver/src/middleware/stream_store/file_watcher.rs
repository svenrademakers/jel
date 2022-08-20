use log::{debug, info, trace, warn};
use notify::{DebouncedEvent, RecursiveMode, Watcher};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::oneshot::Receiver;

use super::LocalStreamStore;
static EXIT: AtomicBool = AtomicBool::new(false);

impl LocalStreamStore {
    pub(super) fn watch_for_changes(stream_store: Arc<LocalStreamStore>, kill_sig: Receiver<()>) {
        tokio::task::spawn_blocking(move || {
            let (tx, rx) = std::sync::mpsc::channel();
            let mut file_watcher = notify::watcher(tx, Duration::from_secs(3)).unwrap();
            if file_watcher
                .watch(&stream_store.root, RecursiveMode::NonRecursive)
                .is_err()
            {
                return;
            }

            debug!(
                "starting watching {} for changes",
                &stream_store.root.to_string_lossy()
            );

            while !EXIT.load(Ordering::Relaxed) {
                if let Ok(event) = rx.recv_timeout(Duration::from_secs(3)) {
                    tokio::spawn(Self::handle_debounce_event(stream_store.clone(), event));
                }
            }

            info!("exiting file watcher");
        });

        tokio::spawn(async move {
            let _ = kill_sig.await;
            debug!("exiting watcher");
            EXIT.store(true, Ordering::Relaxed);
        });
    }

    async fn handle_debounce_event(stream_store: Arc<LocalStreamStore>, event: DebouncedEvent) {
        trace!("received {:?}", &event);
        let result = match event {
            DebouncedEvent::Create(p) => stream_store.load(p).await,
            DebouncedEvent::Write(p) => stream_store.load(p).await,
            DebouncedEvent::Remove(p) => {
                stream_store.removed(p).await;
                Ok(())
            }
            DebouncedEvent::Rename(rem, add) => {
                stream_store.removed(rem).await;
                stream_store.load(add).await
            }
            _ => Ok(()),
        };

        if let Err(e) = result {
            warn!("failed handling event: {}", e);
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use std::{path::PathBuf, time::Duration};

//     use log::{debug, Level};
//     use tempdir::TempDir;
//     use tokio::sync::oneshot::channel;

//     use crate::{
//         logger::init_log,
//         middleware::{
//             interface::{Stream, StreamId, StreamStore},
//             LocalStreamStore,
//         },
//     };

//     fn assert_stream(mut a: Stream, mut b: Stream) {
//         a.sources
//             .iter_mut()
//             .for_each(|f| f.created = std::time::UNIX_EPOCH.into());
//         b.sources
//             .iter_mut()
//             .for_each(|f| f.created = std::time::UNIX_EPOCH.into());
//         assert_eq!(a, b);
//     }

//     #[tokio::test]
//     async fn test_1() {
//         init_log(Level::Debug);
//         let temp = TempDir::new("test").unwrap();
//         let stream_store = LocalStreamStore::new(temp.path()).await;
//         let (kill_sender, kill_recv) = channel();
//         LocalStreamStore::watch_for_changes(stream_store.clone(), kill_recv);

//         tokio::time::sleep(Duration::from_secs(10)).await;

//         stream_store
//             .register(
//                 StreamId::Untagged(2),
//                 vec![PathBuf::from("test2.dash"), PathBuf::from("test_3.m3u8")],
//                 true,
//                 None,
//             )
//             .await
//             .unwrap();

//         assert_stream(
//             Stream {
//                 sources: vec![
//                     Source {
//                         url: "test2.dash".into(),
//                         typ: StreamingType::DASH,
//                         created: std::time::UNIX_EPOCH.into(),
//                     },
//                     Source {
//                         url: "test_3.m3u8".into(),
//                         typ: StreamingType::HLS,
//                         created: std::time::UNIX_EPOCH.into(),
//                     },
//                 ],
//                 live: true,
//             },
//             stream_store.store.read().await[&StreamId::Untagged(2)].clone(),
//         );
//     }
// }
