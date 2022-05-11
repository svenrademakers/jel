mod http_server;
mod logger;
mod services;
mod tls;
mod cli;

use crate::cli::get_config;
use crate::services::MatchService;
use crate::tls::{load_server_config, TlsAcceptor};
use http_server::HttpServer;
use hyper::server::conn::AddrIncoming;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Server};
use log::*;
use logger::init_log;
use std::io;


#[tokio::main]
async fn main() -> io::Result<()> {
    let config = get_config();
    let log_level = match config.verbose {
        true => log::Level::Debug,
        false => log::Level::Info,
    };
    init_log(log_level);

    // load service context data
    let tls_cfg = load_server_config(&config.certificates, &config.private_key)?;
    let mut service_context = HttpServer::new(config.www_dir).await?;
    service_context.append_service("/matches", MatchService::new("2022", "11075"));

    // define how a service is made. when a client connects it will get its own context to talk with
    let make_service = make_service_fn(|_| {
        debug!("handle client");
        let context = service_context.clone();
        async {
            let service = service_fn(move |request| {
                let ctx = context.clone();
                async move { ctx.handle_request(request).await }
            });
            Ok::<_, std::io::Error>(service)
        }
    });

    let addr = format!("{}:{}", config.host, 443).parse().unwrap();
    let incoming = AddrIncoming::bind(&addr).unwrap();
    info!("listening on interface {}", addr);

    let server = Server::builder(TlsAcceptor::new(tls_cfg, incoming)).serve(make_service);
    let redirect_server = redirect_server(&config.hostname, &config.host, config.port);

    let result = tokio::select! {
        res = server => res,
        res = redirect_server => res,
    };

    if let Err(e) = result {
        error!("fatal error. exiting server : {}", e);
    }
    Ok(())
}

async fn redirect_server(hostname: &str, host: &str, port: u16) -> Result<(), hyper::Error> {
    let make_svc = make_service_fn(|_conn| {
        let redirect_location = format!("https://{}", hostname);
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

    let addr = format!("{}:{}", host, port).parse().unwrap();
    let server = Server::bind(&addr).serve(make_svc);
    info!("listening on interface {}", addr);
    server.await
}
