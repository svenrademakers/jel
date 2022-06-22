use std::{
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};

use clap::Parser;
use daemonize::Daemonize;

#[derive(Parser)]
struct Args {
    #[clap(short, long, default_value_t = 7)]
    pub interval_days: u64,
    #[clap(short, long)]
    pub config: PathBuf,
    #[clap(short)]
    pub now: bool,
}

fn main() {
    println!("Hello, world!");
    let args = Args::parse();
    let duration = Duration::from_secs(args.interval_days * 24 * 60 * 60);

    if !args.config.exists() {
        eprintln!("configuration file {:?} does not exist", args.config);
        return;
    }

    let script_path: PathBuf = PathBuf::from("/opt/etc/ronaldos_webserver/uacme.sh");
    if !script_path.exists() {
        eprintln!(
            "cannot renew certificates. {} does not exist",
            script_path.to_string_lossy()
        );
        return;
    }

    let hostname =
        get_hostname_from_config(&args.config).expect("need hostname in order to run acme test");
    let host = format!("www.{}", hostname);

    if args.now {
        execute(&script_path, &host);
        return;
    }

    daemonize();

    loop {
        println!("waking up in {:?}", duration);
        thread::sleep(duration);
        execute(&script_path, &host);
    }
}

fn execute(script_path: &PathBuf, host: &String) {
    let uacme_args = [
        "UACME_CHALLENGE_PATH=/opt/share/ronaldos-website/www",
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

fn get_hostname_from_config(config: &Path) -> Option<String> {
    let data = std::fs::read(config).ok()?;

    if let Ok(serde_yaml::Value::Mapping(x)) = serde_yaml::to_value(&data) {
        let key: serde_yaml::Value = serde_yaml::from_str("hostname").ok()?;
        return x
            .get(&key)
            .and_then(|v| serde_yaml::from_value(v.clone()).ok());
    }
    None
}
