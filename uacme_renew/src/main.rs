mod log;

use ::log::{error, info};
use anyhow::{Context, Result};
use clap::Parser;
use daemonize::Daemonize;
use ronaldos_config::get_webserver_pid;
use std::{
    error::Error,
    fs::File,
    io,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};

#[derive(Parser)]
struct Args {
    #[clap(short, long, default_value = ronaldos_config::CFG_PATH)]
    pub config: PathBuf,
    #[clap(short)]
    pub now: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    log::init().unwrap();
    info!("lets go");
    let cli = Args::parse();
    let config = ronaldos_config::get_application_config(&cli.config);
    let duration = Duration::from_secs(config.interval_days() * 24 * 60 * 60);

    let script_path: PathBuf = PathBuf::from("/opt/share/uacme/uacme.sh");
    if !script_path.exists() {
        error!(
            "{} does not exist. did you install the uacme package?",
            script_path.to_string_lossy()
        );
        return Err(Box::new(io::Error::new(
            io::ErrorKind::NotFound,
            "uacme not found",
        )));
    }

    if cli.now {
        execute(&script_path, config.hostname(), config.www_dir());
        return Ok(());
    }

    daemonize()?;

    loop {
        info!("waking up in {:?}", duration);
        thread::sleep(duration);
        execute(&script_path, config.hostname(), config.www_dir());
    }
}

fn execute(script_path: &Path, host: &String, www: &Path) {
    if get_webserver_pid()
        .expect("this application must know the existence of a pid file")
        .is_none()
        && !webserver_command(false)
    {
        error!("ronaldos_webserver must be running");
        return;
    }

    let challenge_path =
        PathBuf::from_iter([www.to_str().unwrap(), ".well-known", "acme-challenge"]);
    std::fs::create_dir_all(&challenge_path).unwrap();

    match Command::new("uacme")
        .args(["-h", &script_path.to_string_lossy(), "issue", host])
        .env("CHALLENGE_PATH", challenge_path.as_os_str())
        .output()
    {
        Ok(o) => {
            if o.status.success() {
                info!("renew certificates succeeded");
                if !webserver_command(true) {
                    error!("restart of ronaldos_webserver might not be successful");
                }
            } else {
                error!("renew of certificates returned statuscode {}", o.status);
            }
        }
        Err(e) => {
            error!("error executing uacme process {}", e);
        }
    }
}

fn daemonize() -> Result<()> {
    const STDOUT: &str = concat!("/opt/var/", env!("CARGO_PKG_NAME"));
    const PID: &str = "/opt/var/run/ronaldo_uacme.pid";

    let stdout = create_if_not_exists(format!("{}/daemon.out", STDOUT)).unwrap();
    let stderr = create_if_not_exists(format!("{}/daemon.err", STDOUT)).unwrap();
    Daemonize::new()
        .pid_file(PID)
        //.chown_pid(true)
        .stdout(stdout)
        .stderr(stderr)
        .start()
        .context("daemon error")
}

fn webserver_command(restart: bool) -> bool {
    let start = match restart {
        true => "restart",
        false => "start",
    };

    Command::new("ronaldos_webserver")
        .arg("-d")
        .arg(start)
        .output()
        .map(|o| o.status.success())
        .unwrap_or_default()
}

fn create_if_not_exists<P: AsRef<Path>>(path: P) -> Result<File, io::Error> {
    let path: &Path = path.as_ref();
    let parent = path
        .parent()
        .ok_or(io::Error::new(io::ErrorKind::NotFound, ""))?;
    if !parent.exists() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::File::create(path)
}
