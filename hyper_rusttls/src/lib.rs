pub mod https_connector;
pub mod service;
pub mod tls_client_stream;
pub mod tls_config;
pub mod tls_stream;

use hyper::{
    server::conn::AddrIncoming,
    service::{make_service_fn, service_fn},
    Body, Server,
};
use log::{debug, info};
use service::RequestHandler;
use std::{net::SocketAddr, sync::Arc};
use tokio_rustls::rustls::ServerConfig;

use crate::tls_stream::TlsAcceptor;

macro_rules! make_service {
    ($service_context: ident) => {
        make_service_fn(|_| {
            debug!("handle client");
            let context = $service_context.clone();
            async {
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
    hostname: &str,
    tls_cfg: Option<ServerConfig>,
) -> Result<(), hyper::Error>
where
    T: RequestHandler + Send + Clone,
{
    if tls_cfg.is_some() && addres.port() == 80 {
        panic!("cannot use port 80 for tls. its reserved for redirecting http traffic");
    }
    info!("listening on interface {}", addres);

    let incoming = AddrIncoming::bind(&addres).unwrap();
    let result;
    if let Some(cfg) = tls_cfg {
        let make_service = make_service!(service_context);
        let server = Server::builder(TlsAcceptor::new(Arc::new(cfg), incoming)).serve(make_service);
        let mut redirect = addres;
        redirect.set_port(80);
        let redirect_server = redirect_server(hostname, redirect);
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

async fn redirect_server(hostname: &str, addres: SocketAddr) -> Result<(), hyper::Error> {
    let make_svc = make_service_fn(|_conn| {
        let redirect_location = format!("https://www.{}", hostname);
        let service = service_fn(move |req| {
            let location = redirect_location.clone();
            async move {
                debug!("redirecting {} to {}", req.uri(), &location);
                http::Response::builder()
                    .status(http::StatusCode::MOVED_PERMANENTLY)
                    .header("Location", location)
                    .body(Body::empty())
            }
        });
        async { Ok::<_, std::io::Error>(service) }
    });

    let server = Server::bind(&addres).serve(make_svc);
    info!("listening on interface {}", addres);
    server.await
}
