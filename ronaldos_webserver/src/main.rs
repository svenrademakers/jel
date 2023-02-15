mod handlers;
mod logger;
mod middleware;
mod services;
use actix_files::{Files, NamedFile};
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer};
use anyhow::Context;
use clap::{Parser, ValueEnum};
#[cfg(not(windows))]
use daemonize::Daemonize;
use hyper_rusttls::tls_config::load_server_config;
//use handlers::authentication::RonaldoAuthentication;
//use handlers::redirect_service::RedirectScheme;
use log::info;
use logger::init_log;
use ronaldos_config::{get_application_config, Config};

use std::io::ErrorKind;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Command;

use middleware::LocalStreamStore;

use crate::handlers::redirect_service::{self, RedirectScheme};
use crate::middleware::FootballApi;
use crate::services::stream_service::stream_service_config;

/// CLI structure that loads the commandline arguments. These arguments will be
/// serialized in this structure
#[derive(Parser, Default, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(short, long, default_value = ronaldos_config::CFG_PATH )]
    pub config: PathBuf,
    #[clap(short, value_enum)]
    pub daemon: Option<DeamonAction>,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum DeamonAction {
    START,
    STOP,
    RESTART,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = get_application_config(&cli.config);
    let log_level = match config.verbose() {
        true => log::Level::Debug,
        false => log::Level::Info,
    };
    init_log(log_level);

    #[cfg(not(windows))]
    if let Some(option) = cli.daemon {
        daemonize(option).ok_or(std::io::Error::new(ErrorKind::Other, "fatal"))?;
    }

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move { application_main(web::Data::new(config)).await })
}

async fn application_main(config: web::Data<Config>) -> anyhow::Result<()> {
    let mut recordings_disk = LocalStreamStore::new(config.video_dir()).await;
    LocalStreamStore::run(&mut recordings_disk);
    let stream_store = web::Data::from(recordings_disk);
    let football_api =
        web::Data::new(FootballApi::new("2022", "1853", config.api_key().clone()).await);

    // let service_manager = match config.login().username.is_empty() {
    //     false => Some(SessionMananger::new(config.login())),
    //     true => None,
    // };

    let addr_str = format!("{}:{}", config.host(), config.port());
    let sock_address: SocketAddr = addr_str
        .parse()
        .with_context(|| format!("could not parse {} to socket sock_address", addr_str))?;
    let tls_cfg = load_server_config(config.certificates(), config.private_key());
    let tls_enabled = tls_cfg.is_ok();
    let cfg = config.clone();

    let index_file = cfg.www_dir().join("index.html");
    assert!(index_file.exists());

    let mut server = HttpServer::new(move || {
        App::new()
            .wrap(RedirectScheme::new(tls_enabled))
            .app_data(cfg.clone())
            .app_data(football_api.clone())
            .configure(|cfg| stream_service_config(cfg, stream_store.clone()))
            .service(
                Files::new("/", cfg.www_dir())
                    .show_files_listing()
                    .index_file(index_file.to_string_lossy()),
            )
            .service(redirect_favicon)
    });

    let tls_enabled = if tls_enabled { "enabled" } else { "disabled" };
    info!("starting server on {:?} tls={} ", sock_address, tls_enabled);

    if let Ok(cfg) = tls_cfg {
        server = server.bind_rustls(sock_address, cfg)?;
    };
    server = server.bind(sock_address)?;
    server.run().await.context("runtime error")
}

#[get("/favicon.ico")]
async fn redirect_favicon(request: HttpRequest, cfg: web::Data<Config>) -> HttpResponse {
    NamedFile::open_async(format!(
        "{}/images/favicon.ico",
        cfg.www_dir().to_string_lossy()
    ))
    .await
    .unwrap()
    .into_response(&request)
}

#[cfg(not(windows))]
fn daemonize(option: DeamonAction) -> Option<()> {
    const STDOUT: &str = concat!("/opt/var/", env!("CARGO_PKG_NAME"));
    std::fs::create_dir_all(STDOUT).unwrap();

    //let stdout = std::fs::File::create(format!("{}/daemon.out", STDOUT)).unwrap();
    let stderr = std::fs::File::create(format!("{}/daemon.err", STDOUT)).unwrap();

    match option {
        DeamonAction::START => Daemonize::new()
            .pid_file(ronaldos_config::PID)
            .chown_pid_file(true)
            .group("root")
            .user("admin")
            // .stdout(stdout)
            .stderr(stderr)
            .start()
            .ok(),
        DeamonAction::STOP => {
            let pid = std::fs::read(ronaldos_config::PID).ok()?;
            Command::new("/bin/bash")
                .arg("kill")
                .arg(std::str::from_utf8(&pid).ok()?)
                .output()
                .ok()?;
            None
        }
        DeamonAction::RESTART => {
            let _ = daemonize(DeamonAction::STOP);
            daemonize(DeamonAction::START)
        }
    }
}
