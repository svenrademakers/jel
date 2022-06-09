mod cli;
mod http_server;
mod logger;
mod middleware;
mod services;

use crate::cli::Config;
use crate::middleware::{FootballApi, RecordingsOnDisk};
use crate::services::{FileService, FixtureService, RecordingsService, SessionMananger};
use http_server::HttpServer;
use hyper_rusttls::run_server;
use hyper_rusttls::tls_config::load_server_config;
use log::*;
use logger::init_log;
use std::io;
use std::sync::Arc;

#[tokio::main]
async fn main() -> io::Result<()> {
    let config = Config::load();
    let log_level = match config.verbose() {
        true => log::Level::Debug,
        false => log::Level::Info,
    };

    init_log(log_level);
    debug!("loaded:\n {:#?}", config);

    let mut recordings_root = config.www_dir().clone();
    recordings_root.push("recordings");
    let recordings_disk = Arc::new(RecordingsOnDisk::new(recordings_root).await);
    let football_api = Arc::new(
        FootballApi::new(
            "2022",
            "11075",
            config.api_key().clone(),
            recordings_disk.clone(),
        )
        .await,
    );

    let mut service_context = HttpServer::new(config.www_dir()).await?;
    service_context.append_service(FixtureService::new(football_api, recordings_disk.clone()));
    service_context.append_service(FileService::new(config.www_dir()).await?);
    service_context.append_service(SessionMananger::new());
    service_context.append_service(RecordingsService::new(recordings_disk));

    let address = format!("{}:{}", config.host(), config.port())
        .parse()
        .unwrap();
    let tls_cfg = load_server_config(&config.certificates(), &config.private_key());

    if let Err(e) = run_server(service_context, address, tls_cfg.ok()).await {
        error!("error running server: {}", e);
    }

    Ok(())
}
