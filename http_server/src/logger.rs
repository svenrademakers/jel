#![allow(dead_code)]
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::PathBuf;

use log::Metadata;
use log::Record;
use log::{info, Level, LevelFilter};
use syslog::{BasicLogger, Facility, Formatter3164};

/// setup logging component. logs to terminal in debug mode. otherwise to syslog
pub fn init_log(max_log_level: LevelFilter) {
    let box_logger_result;
    if cfg!(debug_assertions) {
        box_logger_result = log::set_boxed_logger(Box::new(TerminalLogger::new(max_log_level)));
    } else {
        let formatter = Formatter3164 {
            facility: Facility::LOG_USER,
            hostname: None,
            process: env!("CARGO_PKG_NAME").into(),
            pid: std::process::id(),
        };

        match syslog::unix(formatter).map(BasicLogger::new) {
            Ok(logger) => box_logger_result = log::set_boxed_logger(Box::new(logger)),
            Err(e) => {
                println!("cannot setup syslog component. not logging {}", e);
                return;
            }
        }
    }

    box_logger_result
        .map(|()| log::set_max_level(max_log_level))
        .expect("logger must be initialized successfully");
    info!("initialized logger with max_level {}", max_log_level);
}

macro_rules! forward_log {
    ($level: expr, $fmt: literal, $($args: expr),*) => {
        if $level == Level::Error {
            eprintln!($fmt, $level, $($args),*);
        } else {
            println!($fmt, $level, $($args),*);
        }
    };
}

struct TerminalLogger {
    max_log_level: LevelFilter,
}

impl TerminalLogger {
    fn new(max_log_level: LevelFilter) -> Self {
        TerminalLogger { max_log_level }
    }
}

impl log::Log for TerminalLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.max_log_level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        forward_log!(
            record.level(),
            "[{}] {} - {}",
            extract_component_name(record.file()).to_string_lossy(),
            record.args()
        );
    }

    fn flush(&self) {}
}

fn extract_component_name(filename: Option<&str>) -> OsString {
    let path = filename.map(PathBuf::from);

    match path
        .as_deref()
        .and_then(std::path::Path::file_stem)
        .and_then(OsStr::to_str)
    {
        Some("mod") => path
            .unwrap()
            .parent()
            .and_then(std::path::Path::file_name)
            .map(ToOwned::to_owned)
            .unwrap_or_default(),
        Some(name) => name.into(),
        None => return "".into(),
    }
}
