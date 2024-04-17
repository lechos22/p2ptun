use iroh_net::NodeAddr;
use tokio::sync::mpsc;

use crate::daemon::packet::Packet;

use super::{peer_collection::PeerCollectionMessage, Actor, Addr};

#[derive(Debug, Clone)]
pub enum PeerSourceMessage {
    DialPeer(NodeAddr),
}
pub struct PeerSource {
    address: Addr<PeerSourceMessage>,
    receiver: mpsc::Receiver<PeerSourceMessage>,
    peers_message_addr: Addr<PeerCollectionMessage>,
    peers_packet_addr: Addr<Packet>,
}
impl PeerSource {
    pub fn new<T>(peer_collection: &T) -> Self
    where
        T: Actor<PeerCollectionMessage> + Actor<Packet>,
    {
        let (sender, receiver) = mpsc::channel(16);
        Self {
            address: Addr::new(sender),
            receiver,
            peers_message_addr: peer_collection.get_addr(),
            peers_packet_addr: peer_collection.get_addr(),
        }
    }
    pub async fn run(mut self) {
        loop {
            let message = match self.receiver.recv().await {
                Some(message) => message,
                None => continue,
            };
            match message {
                PeerSourceMessage::DialPeer(node_addr) => {
                    self.dial_peer(node_addr).await;
                }
            }
        }
    }
    async fn dial_peer(&self, node_addr: NodeAddr) {
        let _ = &self.peers_packet_addr;
        let _ = &self.peers_message_addr;
        let _ = node_addr;
        todo!();
    }
}
impl Actor<PeerSourceMessage> for PeerSource {
    fn get_addr(&self) -> super::Addr<PeerSourceMessage> {
        self.address.clone()
    }
}
