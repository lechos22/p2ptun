use interprocess::local_socket::{
    tokio::{Listener, Stream},
    traits::tokio::{Listener as _, Stream as _},
    ListenerOptions,
};

use iroh_net::{ticket::NodeTicket, NodeId};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::{common::{local_socket_name, DaemonProcedure}, daemon::error::DaemonError};

use super::{peer_collection::PeerCollectionMessage, peer_source::PeerSourceMessage, Addr};

pub struct DaemonController {
    socket: Listener,
    peer_source: Addr<PeerSourceMessage>,
    peer_collection: Addr<PeerCollectionMessage>,
}

impl DaemonController {
    pub fn new(
        peer_source: Addr<PeerSourceMessage>,
        peer_collection: Addr<PeerCollectionMessage>,
    ) -> Result<Self, DaemonError> {
        let socket = ListenerOptions::new().name(local_socket_name()?).create_tokio()?;
        Ok(Self {
            socket,
            peer_source,
            peer_collection,
        })
    }
    pub async fn run(self) {
        while let Ok(stream) = self.socket.accept().await {
            let handler = ConnectionHandler {
                stream,
                peer_source: self.peer_source.clone(),
                peer_collection: self.peer_collection.clone(),
            };
            tokio::spawn(handler.run());
        }
    }
}

struct ConnectionHandler {
    stream: Stream,
    peer_source: Addr<PeerSourceMessage>,
    peer_collection: Addr<PeerCollectionMessage>,
}

impl ConnectionHandler {
    async fn run(self) {
        let (receiver, _responder) = self.stream.split();
        let mut lines = BufReader::new(receiver).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let procedure = match line.parse::<DaemonProcedure>() {
                Ok(procedure) => procedure,
                Err(error) => {
                    eprintln!("Error when parsing a daemon procedure: {:?}", error);
                    continue;
                }
            };
            if let Err(error) =
                Self::execute_procedure(&self.peer_source, &self.peer_collection, procedure).await
            {
                eprintln!("Error when executing a daemon procedure: {:?}", error)
            }
        }
    }
    async fn execute_procedure(
        peer_source: &Addr<PeerSourceMessage>,
        peer_collection: &Addr<PeerCollectionMessage>,
        procedure: DaemonProcedure,
    ) -> Result<(), DaemonError> {
        match procedure {
            DaemonProcedure::DialPeer(node_ticket) => {
                let peer_ticket: NodeTicket = node_ticket.parse()?;
                peer_source
                    .send_message(PeerSourceMessage::DialPeer(peer_ticket.node_addr().clone()))
                    .await;
            }
            DaemonProcedure::DisconnectPeer(node_id) => {
                let node_id: NodeId = node_id.parse()?;
                peer_collection
                    .send_message(PeerCollectionMessage::RemovePeer(node_id))
                    .await;
            }
        }
        Ok(())
    }
}
