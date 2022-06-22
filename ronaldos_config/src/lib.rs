use clap::{ArgEnum, Parser};
use serde::Deserialize;
use std::fmt::Debug;
use std::path::PathBuf;

const CFG_PATH: &str = concat!("/opt/etc/ronaldos-webserver/config.cfg");
const WWW_DEFAULT: &str = concat!("/opt/share/ronaldos-webserver/www");

/// CLI structure that loads the commandline arguments. These arguments will be
/// serialized in this structure
#[derive(Parser, Default, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(short, long, default_value = CFG_PATH )]
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

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Login {
    pub username: String,
    pub password: String,
}

macro_rules! config_definitions {
    ($($name:ident : $type:ty, $default:expr),+) => {
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
    www_dir: PathBuf,
    PathBuf::from(format!("{}/www", WWW_DEFAULT)),
    port: u16,
    80,
    host: String,
    "0.0.0.0".to_string(),
    private_key: PathBuf,
    PathBuf::from("../test_certificates/server.key"),
    certificates: PathBuf,
    PathBuf::from("../test_certificates/server.crt"),
    verbose: bool,
    false,
    api_key: String,
    String::new(),
    video_dir: PathBuf,
    PathBuf::from(format!("{}/videos", WWW_DEFAULT)),
    login: Login,
    Default::default(),
    hostname: String,
    String::new()
);

pub fn get_application_config() -> (Config, Cli) {
    let cli = Cli::parse();
    (Config::load(&cli.config), cli)
}
