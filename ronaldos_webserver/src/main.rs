mod logger;
mod middleware;
mod services;
mod tls_config;

use crate::middleware::{FootballApi, SessionMananger};
use crate::services::authentication_service::RonaldoAuthentication;
use crate::services::fixture_service::fixture_service_config;
use crate::services::redirect_service::RedirectScheme;
use crate::services::stream_service::stream_service_config;
use crate::tls_config::load_server_config;
use actix_files::Files;
use actix_web::{web, App, HttpServer};
use anyhow::{ensure, Context};
use clap::Parser;
use log::info;
use logger::init_log;
use middleware::LocalStreamStore;
use ronaldos_config::{get_application_config, Config};
use rustls::RootCertStore;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

/// CLI structure that loads the commandline arguments. These arguments will be
/// serialized in this structure
#[derive(Parser, Default, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(short, long, default_value = ronaldos_config::CFG_PATH )]
    pub config: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = get_application_config(&cli.config);
    let log_level = match config.verbose() {
        true => log::Level::Debug,
        false => log::Level::Info,
    };
    init_log(log_level);

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move { application_main(web::Data::new(config)).await })
}

async fn application_main(config: web::Data<Config>) -> anyhow::Result<()> {
    let cert_store = Arc::new(native_cert_store());
    let mut recordings_disk = LocalStreamStore::new(config.video_dir()).await;
    LocalStreamStore::run(&mut recordings_disk);
    let stream_store = web::Data::from(recordings_disk);
    let football_api = web::Data::new(
        FootballApi::new("2022", "1853", config.api_key().clone(), cert_store).await,
    );

    let viewer_credentials_set = !config.login().username.is_empty();
    let session_mananger = viewer_credentials_set.then(|| SessionMananger::new(config.login()));

    let tls_cfg = load_server_config(config.certificates(), config.private_key());
    let tls_enabled = tls_cfg.is_ok();

    let index_file = config.www_dir().join("index.html");
    assert!(index_file.exists());

    let cfg = config.clone();
    let mut server = HttpServer::new(move || {
        App::new()
            .app_data(cfg.clone())
            .wrap(RonaldoAuthentication::new(session_mananger.clone()))
            .wrap(RedirectScheme::new(tls_enabled))
            .configure(|cfg| stream_service_config(cfg, stream_store.clone()))
            .configure(|cfg| fixture_service_config(cfg, football_api.clone()))
            .service(web::redirect("/favicon.ico", "/images/favicon.ico"))
            .default_service(
                Files::new("/", cfg.www_dir()).index_file(index_file.to_string_lossy()),
            )
    });

    // if tls is configured, we will use port 80 to redirect people to the
    // secure port
    let port = match tls_enabled {
        true => 80,
        false => *config.port(),
    };

    let addr_str = format!("{}:{}", config.host(), port);
    let sock_address: SocketAddr = addr_str
        .parse()
        .with_context(|| format!("could not parse {} to socket sock_address", addr_str))?;

    if let Ok(cfg) = tls_cfg {
        ensure!(
            config.port() != &80,
            "port 80 is used to run redirect server"
        );
        let secure_address: SocketAddr = format!("{}:{}", config.host(), config.port())
            .parse()
            .with_context(|| format!("could not parse {} to socket sock_address", addr_str))?;
        info!("starting TLS server on {:?}", secure_address);
        server = server.bind_rustls_021(secure_address, cfg)?;
    };

    info!("starting server on {:?}", sock_address);
    server = server.bind(sock_address)?;
    server.run().await.context("runtime error")
}

fn native_cert_store() -> RootCertStore {
    let mut roots = rustls::RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().expect("could not load platform certs") {
        roots.add(&rustls::Certificate(cert.0)).unwrap();
    }
    roots
}
