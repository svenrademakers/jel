mod logger;
mod middleware;
mod root_service;
mod services;

use crate::middleware::{FootballApi, LocalStreamStore};
use crate::services::{FileService, FixtureService, RecordingsService, SessionMananger};
use clap::{ArgEnum, Parser};
use daemonize::Daemonize;
use hyper_rusttls::run_server;
use hyper_rusttls::tls_config::load_server_config;
use log::*;
use logger::init_log;
use ronaldos_config::{get_application_config, Config};
use root_service::RootService;
use std::io::{self, Error, ErrorKind};
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
    #[clap(short, arg_enum)]
    pub daemon: Option<DeamonAction>,
}

#[derive(ArgEnum, Clone, Debug)]
pub enum DeamonAction {
    START,
    STOP,
    RESTART,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let config = get_application_config(&cli.config);
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
    let mut recordings_disk = LocalStreamStore::new(config.video_dir()).await;
    LocalStreamStore::run(&mut recordings_disk);

    let football_api = Arc::new(
        FootballApi::new(
            "2022",
            "11075",
            config.api_key().clone(),
            recordings_disk.clone(),
        )
        .await,
    );

    let service_manager = match config.login().username.is_empty() {
        false => Some(SessionMananger::new(config.login())),
        true => None,
    };
    let mut service_context = RootService::new(config.www_dir(), service_manager).await?;
    service_context.append_service(FixtureService::new(football_api, recordings_disk.clone()));
    service_context.append_service(FileService::new(config.www_dir()).await?);
    service_context.append_service(RecordingsService::new(recordings_disk, *config.verbose()));
    let address = format!("{}:{}", config.host(), config.port())
        .parse()
        .unwrap();
    let tls_cfg = load_server_config(&config.certificates(), &config.private_key());

    if let Err(e) = run_server(
        Arc::new(service_context),
        address,
        config.hostname(),
        tls_cfg.ok(),
    )
    .await
    {
        error!("error running server: {}", e);
    }
    Ok(())
}

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
