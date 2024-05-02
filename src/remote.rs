use std::io::Write;

use interprocess::local_socket::{traits::Stream as _, Stream};

use crate::common::{local_socket_name, DaemonProcedure};

#[derive(Debug)]
pub enum RemoteError {
    IoError(std::io::Error),
}

impl From<std::io::Error> for RemoteError {
    fn from(error: std::io::Error) -> Self {
        Self::IoError(error)
    }
}

fn send_procedure(daemon_procedure: DaemonProcedure) -> Result<(), RemoteError> {
    let mut stream = Stream::connect(local_socket_name()?)?;
    writeln!(stream, "{}", daemon_procedure.to_string())?;
    stream.flush()?;
    Ok(())
}

pub fn dial_peer(node_ticket: &str) -> Result<(), RemoteError> {
    send_procedure(DaemonProcedure::DialPeer(node_ticket.to_string()))
}
pub fn disconnect_peer(node_id: &str) -> Result<(), RemoteError> {
    send_procedure(DaemonProcedure::DisconnectPeer(node_id.to_string()))
}
