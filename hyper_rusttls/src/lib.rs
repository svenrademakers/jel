pub mod https_connector;
pub mod service;
pub mod tls_client_stream;
pub mod tls_config;
pub mod tls_stream;

use crate::tls_stream::TlsAcceptor;
use hyper::{
    server::conn::AddrIncoming,
    service::{make_service_fn, service_fn},
    Body, Server,
};
use log::{debug, info, trace};
use service::RequestHandler;
use std::{net::SocketAddr, sync::Arc};
use tokio_rustls::rustls::ServerConfig;

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
    service_context: Arc<T>,
    addres: SocketAddr,
    hostname: &str,
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
        let redirect_location = format!("https://{}", hostname);
        let service = service_fn(move |req| {
            let location = redirect_location.clone();
            async move {
                let redirect = format!("{}{}", location, req.uri());
                debug!("redirecting {} to {}", req.uri(), &redirect);
                http::Response::builder()
                    .status(http::StatusCode::MOVED_PERMANENTLY)
                    .header("Location", redirect)
                    .body(Body::empty())
            }
        });
        async { Ok::<_, std::io::Error>(service) }
    });

    let server = Server::bind(&addres).serve(make_svc);
    info!("listening on interface {}", addres);
    server.await
}
