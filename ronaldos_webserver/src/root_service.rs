use crate::services::{first_segment_uri, SessionMananger};
use http::{HeaderMap, Request};
use hyper::Body;
use hyper_rusttls::service::RequestHandler;
use log::{debug, info, trace};
use std::fmt::Display;
use std::io;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone)]
pub struct RootService {
    services: BTreeMap<&'static str, Arc<dyn RequestHandler>>,
    session_manager: Option<Arc<SessionMananger>>,
}

#[async_trait::async_trait]
impl RequestHandler for RootService {
    async fn invoke(&self, request: http::Request<Body>) -> std::io::Result<http::Response<Body>> {
        let permission = self.verify_permissions(&request).await;
        let extra_headers = match permission {
            Ok(headers) => headers,
            Err(response) => return Ok(response),
        };


        let path = first_segment_uri(&request).unwrap_or("/");
        let handler = self
            .services
            .get(path)
            .or_else(|| self.services.get(""))
            .expect("there should should always be a default http handler defined");

        debug!(
            "handling request '{}' {} using {}",
            &request.uri(),
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
        "/"
    }
}

impl RootService {
    pub async fn new(
        www_dir: &std::path::Path,
        session_manager: Option<SessionMananger>,
    ) -> io::Result<Self> {
        if !www_dir.exists() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("{} does not exists", www_dir.to_string_lossy()),
            ));
        }
        let session_manager = session_manager.map(Arc::new);
        let mut server = RootService {
            services: BTreeMap::new(),
            session_manager: session_manager.clone(),
        };

        if let Some(sm) = session_manager {
            server.services.insert(SessionMananger::path(), sm);
        }

        Ok(server)
    }

    pub fn append_service<T>(&mut self, service: T)
    where
        T: 'static + RequestHandler,
    {
        info!("added service {}", service);
        self.services.insert(T::path(), Arc::new(service));
    }

    async fn verify_permissions(
        &self,
        request: &Request<Body>,
    ) -> Result<Option<HeaderMap>, http::Response<Body>> {
        if self.session_manager.is_some() {
            match self
                .session_manager
                .as_ref()
                .unwrap()
                .has_permission(&request)
                .await
            {
                Err(denied_response) => {
                    debug!(
                        "{} for {} {}",
                        denied_response.status(),
                        request.uri().path(),
                        request.method()
                    );
                    trace!("request headers: {:?}", request.headers());
                    return Err(denied_response);
                }
                Ok(headers) => return Ok(headers),
            }
        }
        Ok(None)
    }
}

impl Display for RootService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "http Server")
    }
}
