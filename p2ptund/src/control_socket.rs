use std::{net::{Ipv6Addr, SocketAddr, SocketAddrV6}, sync::Arc};
use tokio::net::TcpSocket;

use crate::application::ApplicationState;

const CONTROL_SOCKET_ADDRESS: SocketAddr =
    SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, 2233, 0, 0));

pub struct ControlSocket {
    socket: TcpSocket,
    state: Arc<ApplicationState>
}

impl ControlSocket {
    pub async fn new(state: Arc<ApplicationState>) -> anyhow::Result<ControlSocket> {
        Ok(ControlSocket {
            socket: TcpSocket::new_v6()?,
            state
        })
    }
    pub async fn listen(self) -> anyhow::Result<()> {
        self.socket.bind(CONTROL_SOCKET_ADDRESS)?;
        let listener = self.socket.listen(4)?;
        loop {
            let (stream, addr) = listener.accept().await?;
        }
    }
}
