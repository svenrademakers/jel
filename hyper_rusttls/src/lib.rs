pub mod https_connector;
pub mod service;
pub mod tls_client_stream;
pub mod tls_config;
pub mod tls_stream;

use http::uri::Scheme;
use hyper::{
    server::conn::AddrIncoming,
    service::{make_service_fn, service_fn},
    Body, Server,
};
use log::{debug, info, trace};
use service::RequestHandler;
use std::{net::SocketAddr, sync::Arc};
use tokio_rustls::rustls::ServerConfig;

use crate::tls_stream::TlsAcceptor;

macro_rules! make_service {
    ($service_context: ident) => {
        make_service_fn(move |_| {
            trace!("handle client");
            let context = $service_context.clone();
            async move {
                let service = service_fn(move |request| {
                    let ctx = context.clone();
                    async move { ctx.invoke(request).await }
                });
                Ok::<_, std::io::Error>(service)
            }
        })
    };
}

pub async fn run_server<T>(
    service_context: T,
    addres: SocketAddr,
    tls_cfg: Option<ServerConfig>,
) -> Result<(), hyper::Error>
where
    T: 'static + RequestHandler + Send + Sync + Clone,
{
    if tls_cfg.is_some() && addres.port() == 80 {
        panic!("cannot use port 80 for tls. its reserved for redirecting http traffic");
    }
    info!("listening on interface {}", addres);

    let result;
    if let Some(cfg) = tls_cfg {
        let incoming = AddrIncoming::bind(&addres).unwrap();
        let q = service_context;
        let make_service = make_service!(q);
        let server = Server::builder(TlsAcceptor::new(Arc::new(cfg), incoming)).serve(make_service);
        let mut redirect = addres;
        redirect.set_port(80);
        let redirect_server = redirect_server(redirect);
        result = tokio::select! {
            res = server => res,
            res = redirect_server => res,
        };
    } else {
        let make_service = make_service!(service_context);
        let server = Server::bind(&addres).serve(make_service);
        result = server.await;
    }
    result
}

async fn redirect_server(addres: SocketAddr) -> Result<(), hyper::Error> {
    let make_svc = make_service_fn(|_conn| {
        let service = service_fn(move |req| {
            let mut parts = req.uri().clone().into_parts();
            parts.scheme = Some(Scheme::HTTPS);
            let location = http::Uri::from_parts(parts).unwrap();
            async move {
                debug!("redirecting {} to {}", req.uri(), &location);
                http::Response::builder()
                    .status(http::StatusCode::MOVED_PERMANENTLY)
                    .header("Location", location.to_string())
                    .body(Body::empty())
            }
        });
        async move { Ok::<_, std::io::Error>(service) }
    });

    let server = Server::bind(&addres).serve(make_svc);
    info!("listening on interface {}", addres);
    server.await
}
