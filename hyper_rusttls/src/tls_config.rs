use log::info;
use std::{fs, io, path::Path};
use tokio_rustls::rustls;

pub fn load_server_config(
    certificates: &Path,
    private_key: &Path,
) -> Result<rustls::ServerConfig, io::Error> {
    let certs = load_certs(certificates)?;
    let key = load_private_key(private_key)?;
    let mut cfg = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| error(format!("{}", e)))?;
    cfg.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    info!("loaded tls configuration");
    Ok(cfg)
}

pub fn load_certs(filename: &std::path::Path) -> io::Result<Vec<rustls::Certificate>> {
    info!("loading certificates at {}", filename.to_string_lossy());
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

pub fn load_private_key(filename: &std::path::Path) -> io::Result<rustls::PrivateKey> {
    let keyfile = fs::File::open(filename).map_err(|e| {
        error(format!(
            "failed to open {}: {}",
            filename.to_string_lossy(),
            e
        ))
    })?;
    let mut reader = io::BufReader::new(keyfile);

    let keys = rustls_pemfile::pkcs8_private_keys(&mut reader)
        .map_err(|e| error(format!("failed to load private key {}", e)))?;
    if keys.is_empty() {
        info!("falling back on rsa key reader");
        let rsa_keys = rustls_pemfile::rsa_private_keys(&mut reader)
            .map_err(|e| error(format!("failed to load private key {}", e)))?;
        return Ok(rustls::PrivateKey(rsa_keys[0].clone()));
    }

    Ok(rustls::PrivateKey(keys[0].clone()))
}

fn error(err: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}
