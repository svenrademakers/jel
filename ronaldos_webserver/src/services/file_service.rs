use crate::middleware::cache_map::CacheMap;
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::lock::Mutex;
use hyper::Body;
use hyper_rusttls::service::RequestHandler;
use log::info;
use std::fmt::Display;
use std::io;
use std::path::{Path, PathBuf};

use super::lookup_content_type;

pub struct FileService {
    www_dir: PathBuf,
    header: Vec<u8>,
    footer: Vec<u8>,
    cache: Mutex<CacheMap<Path, Bytes, 32>>,
}

impl FileService {
    pub async fn new(www_dir: &Path) -> io::Result<Self> {
        let mut header_file = PathBuf::from(&www_dir);
        header_file.push("header.html");
        let header = tokio::fs::read(header_file);

        let mut footer_file = PathBuf::from(&www_dir);
        footer_file.push("footer.html");
        let footer = tokio::fs::read(footer_file);

        let (f, h) = tokio::join!(footer, header);

        Ok(FileService {
            www_dir: www_dir.to_path_buf(),
            header: h?,
            footer: f?,
            cache: Mutex::new(CacheMap::new()),
        })
    }

    async fn load_file_from_uri(&self, uri: &str) -> Result<http::Response<Body>> {
        let mut path = self.www_dir.clone();
        path.push(&uri[1..]);
        let bytes;

        let mut lock = self.cache.lock().await;
        if let Some(b) = lock.get(&path) {
            bytes = b.clone();
        } else {
            let content = tokio::fs::read(&path).await?;
            if path.extension().eq(&Some(std::ffi::OsStr::new("html"))) {
                let mut data = Vec::new();
                data.reserve(self.header.len() + self.footer.len() + content.len());
                data.extend_from_slice(&self.header);
                data.extend(content);
                data.extend_from_slice(&self.footer);
                bytes = Bytes::from(data);
            } else {
                bytes = Bytes::from(content);
            }
            lock.insert(&path, bytes.clone());
        }

        drop(lock);
        let mut response = http::Response::builder()
            .status(http::StatusCode::OK)
            .header(http::header::CONTENT_LENGTH, bytes.len());

        if let Some(content_type) = lookup_content_type(&path) {
            response = response.header(http::header::CONTENT_TYPE, content_type);
        }

        Ok(response.body(bytes.into()).unwrap())
    }
}

#[async_trait]
impl RequestHandler for FileService {
    async fn invoke(&self, request: http::Request<Body>) -> std::io::Result<http::Response<Body>> {
        let mut path = request.uri().path();
        if path.eq("/") {
            path = "/index.html";
        }

        self.load_file_from_uri(path).await.or_else(|_| {
            Ok(http::Response::builder()
                .status(http::StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap())
        })
    }

    fn path() -> &'static str {
        ""
    }
}

impl Display for FileService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FileService")
    }
}
