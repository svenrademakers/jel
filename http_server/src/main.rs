mod http_server;
mod redirect_server;
mod services;
mod tls_stream;

use crate::tls_stream::TlsAcceptor;
use clap::Parser;
use http_server::HttpServer;
use hyper::server::conn::AddrIncoming;
use hyper::service::{make_service_fn, service_fn};
use hyper::Server;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;
use std::{fs, io};
use tokio_rustls::rustls;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_name = "PATH", default_value = "../../www")]
    www_dir: PathBuf,
    #[clap(short, long, default_value_t = 80)]
    port: u16,
    #[clap(short, long, default_value = "0.0.0.0")]
    host: String,
    #[clap(long, default_value = "127.0.0.1")]
    hostname: String,
    #[clap(long, default_value = "examples/sample.rsa")]
    private_key: PathBuf,
    #[clap(long, default_value = "examples/sample.pem")]
    certificates: PathBuf,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let args = Args::parse();
    let tls_cfg = load_server_config(&args)?;

    let service_context = HttpServer::new(args.www_dir).await?;
    let make_service = make_service_fn(|_| {
        let context = service_context.clone();
        async {
            let service = service_fn(move |request| {
                let ctx = context.clone();
                async move { ctx.handle_request(request).await }
            });
            Ok::<_, std::io::Error>(service)
        }
    });

    let addr = format!("{}:{}", args.host, args.port).parse().unwrap();
    let incoming = AddrIncoming::bind(&addr).unwrap();
    let server = Server::builder(TlsAcceptor::new(tls_cfg, incoming)).serve(make_service);
    server.await.unwrap();

    Ok(())
}

fn load_server_config(args: &Args) -> Result<Arc<rustls::ServerConfig>, io::Error> {
    let certs = load_certs(&args.certificates)?;
    let key = load_private_key(&args.private_key)?;
    let mut cfg = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| error(format!("{}", e)))?;
    cfg.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(Arc::new(cfg))
}

fn error(err: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

fn load_certs(filename: &std::path::Path) -> io::Result<Vec<rustls::Certificate>> {
    let certfile = fs::File::open(filename).map_err(|e| {
        error(format!(
            "failed to open {}: {}",
            filename.to_string_lossy(),
            e
        ))
    })?;
    let mut reader = io::BufReader::new(certfile);

    // Load and return certificate.
    let certs = rustls_pemfile::certs(&mut reader)
        .map_err(|_| error("failed to load certificate".into()))?;
    Ok(certs.into_iter().map(rustls::Certificate).collect())
}

fn load_private_key(filename: &std::path::Path) -> io::Result<rustls::PrivateKey> {
    let keyfile = fs::File::open(filename).map_err(|e| {
        error(format!(
            "failed to open {}: {}",
            filename.to_string_lossy(),
            e
        ))
    })?;
    let mut reader = io::BufReader::new(keyfile);

    let keys = rustls_pemfile::rsa_private_keys(&mut reader)
        .map_err(|_| error("failed to load private key".into()))?;
    if keys.len() != 1 {
        return Err(error("expected a single private key".into()));
    }

    Ok(rustls::PrivateKey(keys[0].clone()))
}
