use crate::services::SessionMananger;
use hyper::Body;
use hyper_rusttls::service::RequestHandler;
use log::{debug, trace};
use std::fmt::Display;
use std::io;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone)]
pub struct HttpServer {
    services: HashMap<&'static str, Arc<dyn RequestHandler>>,
    session_manager: SessionMananger,
}

#[async_trait::async_trait]
impl RequestHandler for HttpServer {
    async fn invoke(&self, request: http::Request<Body>) -> std::io::Result<http::Response<Body>> {
        let extra_headers;
        match self.session_manager.has_permission(&request).await {
            Err(denied_response) => {
                debug!(
                    "{} for {} {}",
                    denied_response.status(),
                    request.uri().path(),
                    request.method()
                );
                trace!("request headers: {:?}", request.headers());
                return Ok(denied_response);
            }
            Ok(headers) => extra_headers = headers,
        }

        let handler = self
            .services
            .get(request.uri().path())
            .or_else(|| self.services.get("/"))
            .expect("there should should always be a default http handler defined");

        debug!(
            "handling request {} {} using {}",
            &request.uri().path(),
            &request.method(),
            handler
        );

        let mut response = handler.invoke(request).await;
        match &mut response {
            Ok(ref mut res) => {
                if let Some(headers) = extra_headers {
                    for header in headers {
                        res.headers_mut().insert(header.0.unwrap(), header.1);
                    }
                }
                debug!(
                    "successful:{} len:{} headers: {:#?}",
                    res.status(),
                    res.headers()
                        .get(http::header::CONTENT_LENGTH)
                        .map(http::HeaderValue::to_str)
                        .map(Result::unwrap_or_default)
                        .unwrap_or(""),
                    res.headers()
                );
            }
            Err(e) => debug!("failed: {}", e),
        }

        response
    }

    fn path() -> &'static str
    where
        Self: Sized,
    {
        todo!()
    }
}

impl HttpServer {
    pub async fn new(www_dir: &std::path::Path) -> io::Result<Self> {
        if !www_dir.exists() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("{} does not exists", www_dir.to_string_lossy()),
            ));
        }

        Ok(HttpServer {
            services: HashMap::new(),
            session_manager: SessionMananger::new(),
        })
    }

    pub fn append_service<T>(&mut self, service: T)
    where
        T: RequestHandler,
    {
        self.services.insert(T::path(), Arc::new(service));
    }
}

impl Display for HttpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "http Server")
    }
}
