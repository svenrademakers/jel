use futures_util::future::BoxFuture;
use http::Uri;
use hyper::{client::HttpConnector, service::Service};
use std::{sync::Arc, task::Poll};
use tokio_rustls::TlsConnector;

use crate::tls_client_stream::TlsClientStream;

#[derive(Clone)]
pub struct HttpsConnector {
    http: HttpConnector,
    tls: TlsConnector,
}

impl HttpsConnector {
    pub fn new() -> Self {
        let mut http = HttpConnector::new();
        http.enforce_http(false);

        let mut root_store = tokio_rustls::rustls::RootCertStore::empty();
        root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
            tokio_rustls::rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));

        let config = tokio_rustls::rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        Self {
            http,
            tls: TlsConnector::from(Arc::new(config)),
        }
    }
}

impl Default for HttpsConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl Service<Uri> for HttpsConnector {
    type Response = TlsClientStream;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        match self.http.poll_ready(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(e.into())),
            Poll::Pending => Poll::Pending,
        }
    }

    fn call(&mut self, req: Uri) -> Self::Future {
        let host = req
            .host()
            .unwrap_or("")
            .trim_matches(|c| c == '[' || c == ']')
            .to_owned();

        let connecting = self.http.call(req);
        let tls = self.tls.clone();

        Box::pin(async move {
            let tcp = connecting.await.unwrap();
            let tls = tls.connect(host.as_str().try_into()?, tcp).await?;
            Ok(TlsClientStream(tls))
        })
    }
}
