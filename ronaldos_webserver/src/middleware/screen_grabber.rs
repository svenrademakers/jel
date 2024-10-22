use anyhow::bail;
use std::{
    ffi::{CStr, CString},
    fs,
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    sync::Once,
};
use tracing::{info, instrument, warn};

static M3U8_EXT: &str = "m3u8";
static INIT_FFMPEG: Once = Once::new();

extern "C" {
    fn screen_grabber_init();
    fn screen_grabber_start(input: *const i8, output: *const i8) -> i32;
    fn screen_grabber_stop_all() -> i32;
}

pub struct ScreenGrabber {
    capture_name: String,
    root: PathBuf,
}

impl ScreenGrabber {
    pub fn new(capture_name: String, root: PathBuf) -> anyhow::Result<Self> {
        INIT_FFMPEG.call_once(|| unsafe {
            screen_grabber_init();
        });

        Ok(Self { capture_name, root })
    }

    #[instrument(skip_all, fields(filestem = file_stem.as_ref()))]
    pub fn start(&self, file_stem: impl AsRef<str>) -> anyhow::Result<()> {
        let segment_folder = self.root.join(file_stem.as_ref());
        fs::create_dir_all(&segment_folder)?;

        let playlist = segment_folder
            .join(file_stem.as_ref())
            .with_extension(M3U8_EXT);
        info!("starting new playlist at {}", playlist.to_string_lossy());

        unsafe {
            let capture_card = CString::new(self.capture_name.clone())?;
            let clist = CString::new(playlist.as_os_str().as_bytes())?;
            if screen_grabber_start(capture_card.as_ptr(), clist.as_ptr()) != 0 {
                bail!("error occurred starting screen grab");
            }
        }
        Ok(())
    }
}

impl Drop for ScreenGrabber {
    fn drop(&mut self) {
        unsafe {
            screen_grabber_stop_all();
        }
    }
}

#[cfg(test)]
mod tests {
    use tracing::Level;

    use super::ScreenGrabber;
    use std::{path::PathBuf, str::FromStr};

    pub fn with_tracing<T>(f: impl FnOnce() -> T) -> T {
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_max_level(Level::DEBUG)
            .compact()
            .finish();
        tracing::subscriber::with_default(subscriber, f)
    }

    #[test]
    fn no_devices() {
        with_tracing(|| {
            let grabber = ScreenGrabber::new(
                "Game Capture HD60 S+".to_string(),
                PathBuf::from_str("/tmp/test").unwrap(),
            )
            .unwrap();
            grabber.start("sven").unwrap();
        });
    }
}
