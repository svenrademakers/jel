use clap::{IntoApp, Parser};
use log::warn;
use std::fmt::Debug;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(short, long)]
    pub config: Option<PathBuf>,
    #[clap(short, long, value_name = "PATH", default_value = "/opt/share/www")]
    pub www_dir: PathBuf,
    #[clap(short, long, default_value_t = 80)]
    pub port: u16,
    #[clap(short, long, default_value = "0.0.0.0")]
    pub host: String,
    #[clap(long, default_value = "127.0.0.1")]
    pub hostname: String,
    #[clap(long, default_value = "../test_certificates/server.key")]
    pub private_key: PathBuf,
    #[clap(long, default_value = "../test_certificates/server.crt")]
    pub certificates: PathBuf,
    #[clap(short, long)]
    pub verbose: bool,
}

impl Cli {
    pub fn set(&mut self, key: &str, val: serde_yaml::Value) {
        match key {
            "www_dir" => self.www_dir = PathBuf::from(val.as_str().unwrap()),
            "port" => self.port = val.as_u64().unwrap().try_into().unwrap(),
            "host" => self.host = val.as_str().unwrap().to_string(),
            "hostname" => self.hostname = val.as_str().unwrap().to_string(),
            "private_key" => self.www_dir = PathBuf::from(val.as_str().unwrap()),
            "certificates" => self.www_dir = PathBuf::from(val.as_str().unwrap()),
            "verbose" => self.verbose = val.as_bool().unwrap(),
            _ => warn!("config key {} does not exists. will not set it", key),
        }
    }
}

pub fn get_config() -> Cli {
    let mut args = Cli::parse();
    let matches = Cli::command().get_matches();

    if let Some(config) = &args.config {
        let raw = std::fs::read_to_string(config).expect("passed config file could not be read");
        let deserialized_map: serde_yaml::Mapping = serde_yaml::from_str(&raw).unwrap();
        for (key, value) in deserialized_map {
            let key = key.as_str().unwrap();
            // occurrences_of does not count default values of arguments. This
            // way we can see if the user passed an argument via the command
            // line. if so, dont overwrite the config file settings
            if matches.occurrences_of(key) == 0 {
                args.set(key, value);
            }
        }
    }
    args
}
