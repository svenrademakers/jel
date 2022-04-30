use hyper::Body;
use std::io;
use std::{collections::HashMap, sync::Arc};

use crate::services::{FileService, MatchService, RequestHandler, SessionMananger};

#[derive(Clone)]
pub struct HttpServer {
    services: HashMap<&'static str, Arc<dyn RequestHandler>>,
    session_manager: SessionMananger,
    host: String,
}

impl HttpServer {
    pub async fn new(www_dir: std::path::PathBuf, host: &str) -> io::Result<Self> {
        let host = format!("https://{}", host);

        let mut services: HashMap<&'static str, Arc<dyn RequestHandler>> = HashMap::new();
        services.insert("/", Arc::new(FileService::new(www_dir).await?));
        services.insert("/matches", Arc::new(MatchService::new("2022", "11075")));
        services.insert("/dologin", Arc::new(SessionMananger::new()));

        Ok(HttpServer {
            services,
            session_manager: SessionMananger::new(),
            host,
        })
    }

    pub async fn handle_request(
        &self,
        mut request: http::Request<Body>,
    ) -> Result<http::Response<Body>, io::Error> {
        self.remove_hostname_from_uri(&mut request);

        if let Some(denied_response) = self.session_manager.has_permission(&request).await {
            return Ok(denied_response);
        }

        let handler = self
            .services
            .get(request.uri().path())
            .or_else(|| self.services.get("/"))
            .expect("there should should always be a default http handler defined");

        let response = handler.invoke(request).await;
        response
    }

    fn remove_hostname_from_uri(&self, request: &mut http::Request<Body>) {
        let uri =
            http::uri::Uri::try_from(request.uri().path().trim_start_matches(&self.host)).unwrap();
        *request.uri_mut() = uri;
    }
}
