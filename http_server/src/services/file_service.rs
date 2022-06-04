use async_trait::async_trait;
use hyper::Body;
use hyper_rusttls::service::RequestHandler;
use std::ffi::OsStr;
use std::fmt::Display;
use std::io;
use std::path::{Path, PathBuf};

pub struct FileService {
    www_dir: PathBuf,
    header: Vec<u8>,
    footer: Vec<u8>,
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
        })
    }

    async fn load_file_from_uri(&self, uri: &str) -> Result<http::Response<Body>, std::io::Error> {
        let mut path = self.www_dir.clone();
        path.push(&uri[1..]);
        let mut bytes = Vec::new();

        let content = tokio::fs::read(&path).await?;
        if path.extension().eq(&Some(std::ffi::OsStr::new("html"))) {
            bytes.reserve(self.header.len() + self.footer.len() + content.len());
            bytes.extend_from_slice(&self.header);
            bytes.extend(content);
            bytes.extend_from_slice(&self.footer);
        } else {
            bytes.extend(content);
        }

        let content_type = match path.extension().and_then(OsStr::to_str) {
            Some("jpeg") => "image/jpeg",
            Some("png") => "image/png",
            Some("svg") => "image/svg+xml",
            Some("json") => "application/json",
            Some("js") => "text/javascript",
            Some("css") => "text/css",
            Some("html" | "htm") => "text/html; charset=UTF-8",
            _ => "application/octet-stream",
        };

        let response = http::Response::builder()
            .status(http::StatusCode::OK)
            .header(http::header::CONTENT_TYPE, content_type)
            .header(http::header::CONTENT_LENGTH, bytes.len())
            .body(bytes.into())
            .unwrap();
        Ok(response)
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
        "/"
    }
}

impl Display for FileService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FileService")
    }
}
