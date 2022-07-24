use log::{Level, Metadata, Record};
use log::{LevelFilter, SetLoggerError};
use std::{fmt::Arguments, process::Command};

static LOGGER: SysLogger = SysLogger;
struct SysLogger;

impl log::Log for SysLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        match record.level() {
            Level::Error => {
                eprintln!("{}", record.args());
                logger_command(record.args(), "-s");
            }
            _ => {
                println!("{}", record.args());
                logger_command(record.args(), "-c");
            }
        }
    }

    fn flush(&self) {}
}

fn logger_command(args: &Arguments, flag: &str) {
    let text = format!("{}", args);
    Command::new("logger")
        .args(["-t", env!("CARGO_PKG_NAME"), &text, flag])
        .output()
        .unwrap();
}

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Info))
}
