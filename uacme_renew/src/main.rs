use clap::Parser;
use daemonize::Daemonize;
use std::{
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

fn main() {
    let cli = Args::parse();
    let config = ronaldos_config::get_application_config(&cli.config);
    let duration = Duration::from_secs(config.interval_days() * 24 * 60 * 60);

    let script_path: PathBuf = PathBuf::from("/opt/share/uacme/uacme.sh");
    if !script_path.exists() {
        eprintln!(
            "{} does not exist. did you install the uacme package?",
            script_path.to_string_lossy()
        );
        return;
    }

    let host = format!("www.{}", config.hostname());

    if cli.now {
        execute(&script_path, &host, config.www_dir());
        return;
    }

    daemonize();

    loop {
        println!("waking up in {:?}", duration);
        thread::sleep(duration);
        execute(&script_path, &host, config.www_dir());
    }
}

fn execute(script_path: &Path, host: &String, www: &Path) {
    let uacme_challenge_path = format!("UACME_CHALLENGE_PATH={}", www.to_string_lossy());
    let uacme_args = [
        &uacme_challenge_path,
        "uacme",
        "-h",
        &script_path.to_string_lossy(),
        "issue",
        host,
    ];
    match Command::new("/bin/bash").args(uacme_args).output() {
        Ok(o) => {
            if o.status.success() {
                println!(" renew certificates succeeded");
                if !Command::new("/bin/bash")
                    .arg("ronaldos_webserver")
                    .arg("-d")
                    .arg("restart")
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or_default()
                {
                    eprintln!("restart of ronaldos_webserver might not be successful");
                }
            } else {
                eprintln!("renew of certificates returned statuscode {}", o.status);
            }
        }
        Err(e) => {
            eprintln!("eprintln executing uacme process {}", e);
        }
    }
}

fn daemonize() {
    const STDOUT: &str = concat!("/opt/var/", env!("CARGO_PKG_NAME"));
    const PID: &str = "/opt/var/run/ronaldo_uacme.pid";
    let stdout = std::fs::File::create(format!("{}/daemon.out", STDOUT)).unwrap();
    let stderr = std::fs::File::create(format!("{}/daemon.err", STDOUT)).unwrap();
    Daemonize::new()
        .pid_file(PID)
        //.chown_pid(true)
        .stdout(stdout)
        .stderr(stderr)
        .start()
        .ok();
}
