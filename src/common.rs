use std::str::FromStr;

use interprocess::local_socket::{GenericNamespaced, Name, ToNsName};

pub enum DaemonProcedure {
    DialPeer(String),
    DisconnectPeer(String),
}

impl ToString for DaemonProcedure {
    fn to_string(&self) -> String {
        match self {
            Self::DialPeer(peer_ticket) => format!("dial_peer {}", peer_ticket),
            Self::DisconnectPeer(peer_id) => format!("disconnect_peer {}", peer_id),
        }
    }
}

#[derive(Debug)]
pub enum ParseError {
    BadDaemonProcedure,
}

impl FromStr for DaemonProcedure {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.split_whitespace().collect::<Vec<&str>>().as_slice() {
            &["dial_peer", peer_ticket] => Ok(DaemonProcedure::DialPeer(peer_ticket.to_string())),
            &["disconnect_peer", peer_id] => {
                Ok(DaemonProcedure::DisconnectPeer(peer_id.to_string()))
            }
            _ => Err(ParseError::BadDaemonProcedure),
        }
    }
}

pub fn local_socket_name() -> std::io::Result<Name<'static>> {
    "/var/run/p2ptun.sock".to_ns_name::<GenericNamespaced>()
}
