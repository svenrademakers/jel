use std::{sync::Arc, time::Duration};

use log::error;
use notify::{DebouncedEvent, RecursiveMode, Watcher};
use tokio::sync::mpsc::{error::TryRecvError, Receiver};

use super::LocalStreamStore;

impl LocalStreamStore {
    pub(super) fn watch_for_changes(
        stream_store: Arc<LocalStreamStore>,
        mut kill_sig: Receiver<()>,
    ) {
        tokio::task::spawn_blocking(move || {
            let (tx, rx) = std::sync::mpsc::channel();
            let mut file_watcher = notify::watcher(tx, Duration::from_secs(3)).unwrap();
            if file_watcher
                .watch(&stream_store.root, RecursiveMode::Recursive)
                .is_err()
            {
                return;
            }

            loop {
                println!("waiting");
                match rx.recv_timeout(Duration::from_secs(30)) {
                    Ok(event) => Self::handle_debounce_event(stream_store.clone(), event),
                    Err(_) => match kill_sig.try_recv() {
                        Ok(_) => return,
                        Err(TryRecvError::Disconnected) => error!("kill channel closed. This cannot happen as the sender is owned in the current scope. "),
                        Err(TryRecvError::Empty) => (),
                    },
                }
            }
        });
    }

    fn handle_debounce_event(stream_store: Arc<LocalStreamStore>, event: DebouncedEvent) {
        println!("{:?}", Arc::strong_count(&stream_store));
    }
}
