use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

pub fn init_logger(log_level: Level) {
    let compact_layer = tracing_subscriber::fmt::layer()
        .without_time()
        .with_ansi(true)
        .with_writer(std::io::stdout)
        .compact();

    let filter = EnvFilter::from_default_env()
        .add_directive(log_level.into())
        .add_directive("rustls=off".parse().unwrap());

    tracing_subscriber::registry()
        .with(compact_layer.with_filter(filter))
        .init();

    tracing::info!(
        "started logging {} {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
}
