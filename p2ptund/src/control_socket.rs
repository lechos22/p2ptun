use std::{
    future::Future,
    net::{Ipv6Addr, SocketAddr, SocketAddrV6},
    pin::Pin,
    sync::Arc,
};

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::{TcpSocket, TcpStream},
};

use crate::application::{Application, ApplicationState};

const CONTROL_SOCKET_ADDRESS: SocketAddr =
    SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, 2233, 0, 0));

pub trait ControlSocketLogic {
    fn listen(self: Arc<Self>) -> Pin<Box<dyn Future<Output = anyhow::Result<()>>>>;
}

enum CommandMessage {
    NoOp,
    AddPeer(String),
}

async fn parse_message(message: String) -> anyhow::Result<CommandMessage> {
    let args: Vec<_> = message.split(' ').collect();
    Ok(match &args[0].to_uppercase() as &str {
        "NOOP" => CommandMessage::NoOp,
        "ADD_PEER" => CommandMessage::AddPeer(args[1].to_string()),
        _ => anyhow::bail!("Unrecognized command"),
    })
}

async fn handle_connection(
    state: Arc<ApplicationState>,
    (stream, addr): (TcpStream, SocketAddr),
) -> anyhow::Result<()> {
    if !addr.ip().is_loopback() {
        return Ok(());
    }
    let mut stream = BufReader::new(stream);
    loop {
        let mut buf = String::new();
        stream.read_line(&mut buf).await?;
        match parse_message(buf).await? {
            CommandMessage::AddPeer(addr) => {
                state.add_peer_from_address(addr);
            }
            CommandMessage::NoOp => {}
        }
    }
}

impl ControlSocketLogic for ApplicationState {
    fn listen(self: Arc<Self>) -> Pin<Box<dyn Future<Output = anyhow::Result<()>>>> {
        Box::pin(async {
            let state = self;
            let socket = TcpSocket::new_v6()?;
            socket.bind(CONTROL_SOCKET_ADDRESS)?;
            let listener = socket.listen(4)?;
            loop {
                tokio::spawn(handle_connection(state.clone(), listener.accept().await?));
            }
        })
    }
}
