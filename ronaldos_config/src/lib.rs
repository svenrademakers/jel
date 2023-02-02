use serde::Deserialize;
use std::fmt::Debug;
use std::io::{self};
use std::path::{Path, PathBuf};

pub const DEFAULT_DATA: &str = concat!("/opt/share/ronaldos-webserver");
pub const CFG_PATH: &str = concat!("/opt/etc/ronaldos-webserver/config.cfg");
pub const PID: &str = "/opt/var/run/ronaldos-webserver.pid";

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Login {
    pub username: String,
    pub password: String,
}

macro_rules! config_definitions {
    ($($name:ident : $type:ty = $default:expr),+) => {
        #[derive(Deserialize, Debug, Default)]
        pub struct Config {
            $($name: Option<$type>,)*
        }

        impl Config {
            pub fn load<P: AsRef<std::path::Path>>(config_file : &P) -> Self {
                let mut cfg  = match std::fs::read_to_string(config_file){
                    Ok(raw) => {
                          serde_yaml::from_str::<Config>(&raw).unwrap()
                    }
                    _ => {
                        eprintln!("could not read config {}. defaulting config", config_file.as_ref().to_string_lossy());
                        Config::default()
                    }
                };

                $(if cfg.$name.is_none() {
                    cfg.$name = Some($default);
                })*

                cfg
            }

           $( pub fn $name(&self) -> &$type {
               self.$name.as_ref().unwrap()
           })*
        }
    };
}

config_definitions!(
    www_dir: PathBuf = PathBuf::from(format!("{}/www", DEFAULT_DATA)),
    port: u16 = 80,
    host: String = "0.0.0.0".to_string(),
    private_key: PathBuf = PathBuf::from("../test_certificates/server.key"),
    certificates: PathBuf = PathBuf::from("../test_certificates/server.crt"),
    verbose: bool = false,
    api_key: String = String::new(),
    video_dir: PathBuf = PathBuf::from(format!("{}/videos", DEFAULT_DATA)),
    login: Login = Default::default(),
    hostname: String = String::from("localhost"),
    interval_days: u64 = 7
);

pub fn get_application_config<P: AsRef<Path>>(config: &P) -> Config {
    Config::load(config)
}

pub fn get_webserver_pid() -> io::Result<Option<u32>> {
    let pid = std::fs::read_to_string(PID)?;
    if !pid.is_empty() {
        pid.parse::<u32>().map(Option::Some).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("pid file unparseable {}", e),
            )
        })
    } else {
        Ok(None)
    }
}
