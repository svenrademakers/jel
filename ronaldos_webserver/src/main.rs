mod handlers;
mod logger;

use actix_files::Files;
use actix_web::{web, App, HttpServer};
use anyhow::Context;
use clap::{Parser, ValueEnum};
#[cfg(not(windows))]
use daemonize::Daemonize;
use hyper_rusttls::tls_config::load_server_config;

//use handlers::authentication::RonaldoAuthentication;
use handlers::redirect_service::RedirectScheme;
use log::info;
use logger::init_log;
use ronaldos_config::{get_application_config, Config};
use std::fmt::format;
use std::io::{self, ErrorKind};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

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
    rt.block_on(async move { application_main(Arc::new(config)).await })
}

async fn application_main(config: Arc<Config>) -> anyhow::Result<()> {
   // let mut recordings_disk = LocalStreamStore::new(config.video_dir()).await;
   // LocalStreamStore::run(&mut recordings_disk);

   // let football_api = FootballApi::new("2022", "1853", config.api_key().clone()).await;

   // let service_manager = match config.login().username.is_empty() {
   //     false => Some(SessionMananger::new(config.login())),
   //     true => None,
   // };

    let addr_str = format!("{}:{}", config.host(), config.port());
    let address: SocketAddr = addr_str
        .parse().with_context(||format!("could not parse {} to socket address", addr_str))?;
    let tls_cfg = load_server_config(config.certificates(), config.private_key());
    let tls_enabled = tls_cfg.is_ok();
    let cfg = config.clone();

    let build_server = HttpServer::new(move || {
        App::new() //.wrap(RedirectSchemeBuilder::new().build())
            .wrap(RedirectScheme::new(tls_enabled))
            //.wrap(RonaldoAuthentication::new())
            .app_data(cfg.clone())
            .service(
                Files::new("/", cfg.www_dir())
                    .index_file(cfg.www_dir().join("index.html").to_string_lossy()),
            )
    });

    let tls_enabled = if tls_enabled { "enabled"} else {"disabled"};
    info!("starting server on {:?} tls={} ", address, tls_enabled );

    let server = match tls_cfg {
        Ok(cfg) => build_server.bind_rustls(address, cfg)?,
        Err(_) => build_server.bind(address)?,
    };
    server.run().await.context("runtime error")
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
