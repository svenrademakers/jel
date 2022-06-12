mod cli;
mod http_server;
mod logger;
mod middleware;
mod services;

use crate::cli::{Cli, Config};
use crate::middleware::{FootballApi, RecordingsOnDisk};
use crate::services::{FileService, FixtureService, RecordingsService, SessionMananger};
use clap::Parser;
use cli::DeamonAction;
use daemonize::Daemonize;
use http_server::HttpServer;
use hyper_rusttls::run_server;
use hyper_rusttls::tls_config::load_server_config;
use log::*;
use logger::init_log;
use std::io::{self, Error, ErrorKind};
use std::process::Command;
use std::sync::Arc;

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let config = Config::load(&cli);
    let log_level = match config.verbose() {
        true => log::Level::Debug,
        false => log::Level::Info,
    };
    init_log(log_level);

    if let Some(option) = cli.daemon {
        let _ = daemonize(option).ok_or(Error::new(ErrorKind::Other, "fatal"))?;
    }

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move { application_main(config).await })
}

async fn application_main(config: Config) -> Result<(), Error> {
    debug!("loaded:\n {:#?}", config);
    let recordings_disk = Arc::new(RecordingsOnDisk::new(config.video_dir()).await);
    let football_api = Arc::new(
        FootballApi::new(
            "2022",
            "11075",
            config.api_key().clone(),
            recordings_disk.clone(),
        )
        .await,
    );
    let service_manager = Some(SessionMananger::new(config.login()));
    let mut service_context = HttpServer::new(config.www_dir(), service_manager).await?;
    service_context.append_service(FixtureService::new(football_api, recordings_disk.clone()));
    service_context.append_service(FileService::new(config.www_dir()).await?);
    service_context.append_service(RecordingsService::new(recordings_disk));
    let address = format!("{}:{}", config.host(), config.port())
        .parse()
        .unwrap();
    let tls_cfg = load_server_config(&config.certificates(), &config.private_key());
    if let Err(e) = run_server(Arc::new(service_context), address, config.hostname(),tls_cfg.ok()).await {
        error!("error running server: {}", e);
    }
    Ok(())
}

fn daemonize(option: DeamonAction) -> Option<()> {
    const PID: &str = "/opt/var/run/ronaldo.pid";
    const STDOUT: &str = concat!("/opt/var/", env!("CARGO_PKG_NAME"));
    std::fs::create_dir_all(STDOUT).unwrap();

    //let stdout = std::fs::File::create(format!("{}/daemon.out", STDOUT)).unwrap();
    let stderr = std::fs::File::create(format!("{}/daemon.err", STDOUT)).unwrap();

    match option {
        DeamonAction::START => Daemonize::new()
            .pid_file(PID)
            .chown_pid(true)
           // .stdout(stdout)
            .stderr(stderr)
            .start()
            .ok(),
        DeamonAction::STOP => {
            let pid = std::fs::read(PID).ok()?;
            Command::new("kill")
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
