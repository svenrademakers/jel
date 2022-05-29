use crate::services::{FileService, RequestHandler, SessionMananger};
use hyper::Body;
use log::{debug, trace};
use std::io;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone)]
pub struct HttpServer {
    services: HashMap<&'static str, Arc<dyn RequestHandler>>,
    session_manager: SessionMananger,
}

impl HttpServer {
    pub async fn new(www_dir: &std::path::Path) -> io::Result<Self> {
        if !www_dir.exists() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("{} does not exists", www_dir.to_string_lossy()),
            ));
        }
        let mut services: HashMap<&'static str, Arc<dyn RequestHandler>> = HashMap::new();
        services.insert("/", Arc::new(FileService::new(www_dir).await?));
        services.insert("/dologin", Arc::new(SessionMananger::new()));

        Ok(HttpServer {
            services,
            session_manager: SessionMananger::new(),
        })
    }

    pub fn append_service<T>(&mut self, uri: &'static str, service: T)
    where
        T: RequestHandler,
    {
        self.services.insert(uri, Arc::new(service));
    }

    pub async fn handle_request(
        &self,
        mut request: http::Request<Body>,
    ) -> Result<http::Response<Body>, io::Error> {
        self.remove_hostname_from_uri(&mut request);
        if let Some(denied_response) = self.session_manager.has_permission(&request).await {
            debug!(
                "{} for {} {}",
                denied_response.status(),
                request.uri(),
                request.method()
            );
            trace!("request headers: {:?}", request.headers());
            return Ok(denied_response);
        }

        let handler = self
            .services
            .get(request.uri().path())
            .or_else(|| self.services.get("/"))
            .expect("there should should always be a default http handler defined");

        debug!(
            "handling request {} {} using {}",
            &request.uri(),
            &request.method(),
            handler
        );

        let response = handler.invoke(request).await;
        match &response {
            Ok(res) => debug!(
                "successful:{} len:{}",
                res.status(),
                res.headers()
                    .get(http::header::CONTENT_LENGTH)
                    .map(http::HeaderValue::to_str)
                    .map(Result::unwrap_or_default)
                    .unwrap_or("")
            ),
            Err(e) => debug!("failed: {}", e),
        }

        response
    }

    fn remove_hostname_from_uri(&self, request: &mut http::Request<Body>) {
        if let Some(authority) = request.uri().authority() {
            let uri = http::uri::Uri::try_from(
                request.uri().path().trim_start_matches(authority.as_str()),
            )
            .unwrap();
            *request.uri_mut() = uri;
        }
    }
}
