use log::info;
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer},
    ServerConfig,
};
use std::{fs, io, path::Path};

pub fn load_server_config(
    certificates: &Path,
    private_key: &Path,
) -> Result<ServerConfig, io::Error> {
    let certs = load_certs(certificates)?;
    let key = load_private_key(private_key)?;
    let mut cfg = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| error(format!("{}", e)))?;
    cfg.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    info!("loaded tls configuration");
    Ok(cfg)
}

pub fn load_certs(filename: &std::path::Path) -> io::Result<Vec<CertificateDer<'static>>> {
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
    let certs = rustls_pemfile::certs(&mut reader).filter_map(|x| x.ok());
    Ok(certs.collect())
}

pub fn load_private_key(filename: &std::path::Path) -> io::Result<PrivateKeyDer<'static>> {
    let keyfile = fs::File::open(filename).map_err(|e| {
        error(format!(
            "failed to open {}: {}",
            filename.to_string_lossy(),
            e
        ))
    })?;
    let mut reader = io::BufReader::new(keyfile);

    let Some(key) = rustls_pemfile::private_key(&mut reader)
        .map_err(|e| error(format!("failed to load private key {}", e)))?
    else {
        return Err(error(format!(
            "no private key found in {}",
            filename.to_string_lossy()
        )));
    };

    Ok(key)
}

fn error(err: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}
