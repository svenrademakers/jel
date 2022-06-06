use async_trait::async_trait;
use hyper::Body;
use hyper_rusttls::service::RequestHandler;
use log::info;
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

        let mut response = http::Response::builder()
            .status(http::StatusCode::OK)
            .header(http::header::CONTENT_LENGTH, bytes.len());

        if let Some(content_type) = lookup_content_type(&path) {
            response = response.header(http::header::CONTENT_TYPE, content_type);
        }

        response = append_additional_headers(&path, response);

        Ok(response.body(bytes.into()).unwrap())
    }
}

fn lookup_content_type(path: &Path) -> Option<&'static str> {
    let content_type = match path.extension().and_then(OsStr::to_str) {
        Some("jpeg") => Some("image/jpeg"),
        Some("png") => Some("image/png"),
        Some("svg") => Some("image/svg+xml"),
        Some("json") => Some("application/json"),
        Some("js") => Some("text/javascript"),
        Some("css") => Some("text/css"),
        Some("html" | "htm") => Some("text/html; charset=UTF-8"),
        _ => None,
    };
    content_type
}

fn append_additional_headers(
    path: &Path,
    builder: http::response::Builder,
) -> http::response::Builder {
    match path.extension().and_then(OsStr::to_str) {
        Some("m3u8" | "ts") => builder
            .header(http::header::CACHE_CONTROL, "no-cache")
            .header(http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "'*' always")
            .header(
                http::header::ACCESS_CONTROL_EXPOSE_HEADERS,
                "Content-Length",
            ),
        _ => builder,
    }
}

fn preflight_reponse() -> http::Response<Body> {
    http::Response::builder()
        .status(http::StatusCode::NO_CONTENT)
        .header(http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(http::header::ACCESS_CONTROL_MAX_AGE, "1728000")
        .header(http::header::CONTENT_TYPE, "text/plain charset=UTF-8")
        .header(http::header::CONTENT_LENGTH, "0")
        .body(Body::empty())
        .unwrap()
}

#[async_trait]
impl RequestHandler for FileService {
    async fn invoke(&self, request: http::Request<Body>) -> std::io::Result<http::Response<Body>> {
        if request.method() == http::Method::OPTIONS {
            info!("preflight for {:?}", request);
            return Ok(preflight_reponse());
        }

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
