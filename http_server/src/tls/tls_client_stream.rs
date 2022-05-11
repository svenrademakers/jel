use std::pin::Pin;

use hyper::client::connect::{Connected, Connection, HttpInfo};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;

/// wrap the tls stream so that we can expose the `Connection` trait, required
/// by hyper
pub struct TlsClientStream(pub TlsStream<TcpStream>);

impl AsyncRead for TlsClientStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let tls_stream = &mut Pin::get_mut(self).0;
        Pin::new(tls_stream).poll_read(cx, buf)
    }
}

impl AsyncWrite for TlsClientStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let tls_stream = &mut Pin::get_mut(self).0;
        Pin::new(tls_stream).poll_write(cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        let tls_stream = &mut Pin::get_mut(self).0;
        Pin::new(tls_stream).poll_flush(cx)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        let tls_stream = &mut Pin::get_mut(self).0;
        Pin::new(tls_stream).poll_shutdown(cx)
    }
}

impl Connection for TlsClientStream {
    fn connected(&self) -> Connected {
        let tcp_stream = self.0.get_ref().0;
        tcp_stream.connected()
    }
}
