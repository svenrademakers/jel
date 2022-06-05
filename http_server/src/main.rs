mod cli;
mod http_server;
mod logger;
mod services;

use crate::cli::Config;
use crate::services::{FileService, MatchService, SessionMananger};
use http_server::HttpServer;
use hyper_rusttls::run_server;
use hyper_rusttls::tls_config::load_server_config;
use log::*;
use logger::init_log;
use std::io;

#[tokio::main]
async fn main() -> io::Result<()> {
    let config = Config::load();
    let log_level = match config.verbose() {
        true => log::Level::Debug,
        false => log::Level::Info,
    };
    init_log(log_level);

    debug!("loaded:\n {:#?}", config);

    // load service context data
    let tls_cfg = load_server_config(&config.certificates(), &config.private_key());
    let mut service_context = HttpServer::new(config.www_dir()).await?;
    service_context.append_service(MatchService::new("2022", "11075", config.api_key().clone()));
    service_context.append_service(FileService::new(config.www_dir()).await?);
    service_context.append_service(SessionMananger::new());

    if let Err(e) = run_server(
        service_context,
        format!("{}:{}", config.host(), config.port())
            .parse()
            .unwrap(),
        config.hostname(),
        tls_cfg.ok(),
    )
    .await
    {
        error!("error running server: {}", e);
    }
    Ok(())
}
