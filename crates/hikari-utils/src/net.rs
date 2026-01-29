use listenfd::ListenFd;
use std::io;
use std::net::{IpAddr, SocketAddr};
use tokio::net::TcpListener;

pub async fn create_listener(
    (host, port): (Option<IpAddr>, Option<u16>),
    (default_host, default_port): (IpAddr, u16),
) -> io::Result<TcpListener> {
    if host.is_none() && port.is_none() {
        let mut listenfd = ListenFd::from_env();
        if let Some(listener) = listenfd.take_tcp_listener(0)? {
            listener.set_nonblocking(true)?;
            tracing::trace!("returning listenfd listener");
            return TcpListener::from_std(listener);
        }
    }

    let ip_addr = host.unwrap_or(default_host);
    let port = port.unwrap_or(default_port);
    let address = SocketAddr::from((ip_addr, port));
    tracing::trace!("returning address {address}:{port}");
    TcpListener::bind(address).await
}
