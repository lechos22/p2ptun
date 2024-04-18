use std::collections::HashMap;

use iroh_net::NodeId;
use tokio::{select, sync::mpsc};

use crate::daemon::packet::Packet;

use super::{peer::Peer, Actor, Addr};

pub enum PeerCollectionMessage {
    AddPeer(NodeId, Peer),
    RemovePeer(NodeId),
}
pub struct PeerCollection {
    message_address: Addr<PeerCollectionMessage>,
    message_receiver: mpsc::Receiver<PeerCollectionMessage>,
    router_address: Addr<Packet>,
    packet_address: Addr<Packet>,
    packet_receiver: mpsc::Receiver<Packet>,
    peers: HashMap<NodeId, Addr<Packet>>,
}
impl PeerCollection {
    pub fn new(router_address: Addr<Packet>) -> Self {
        let (message_sender, message_receiver) = mpsc::channel(16);
        let (packet_sender, packet_receiver) = mpsc::channel(16);
        Self {
            message_address: Addr::new(message_sender),
            message_receiver,
            router_address,
            packet_address: Addr::new(packet_sender),
            packet_receiver,
            peers: HashMap::new(),
        }
    }
    async fn handle_message(&mut self, message: PeerCollectionMessage) {
        match message {
            PeerCollectionMessage::AddPeer(node_id, peer) => {
                self.add_peer(node_id, peer);
            }
            PeerCollectionMessage::RemovePeer(node_id) => {
                self.remove_peer(node_id);
            }
        }
    }
    fn add_peer(&mut self, node_id: NodeId, peer: Peer) {
        println!("Connected to peer {}", node_id);
        self.peers.insert(node_id, peer.get_addr());
        let message_address = self.message_address.clone();
        tokio::spawn(async move {
            peer.run().await;
            println!("Disconnected from peer {}", node_id);
            message_address
                .send_message(PeerCollectionMessage::RemovePeer(node_id))
                .await;
        });
    }
    fn remove_peer(&mut self, node_id: NodeId) {
        self.peers.remove(&node_id);
    }
    async fn handle_packet(&self, packet: Packet) {
        match &packet {
            packet @ Packet::Outgoing(_) => {
                self.send_packet_to_peers(packet).await;
            }
            packet @ Packet::Incoming(_) => {
                self.router_address.send_message(packet.clone()).await;
            }
        }
    }
    async fn send_packet_to_peers(&self, packet: &Packet) {
        for peer in self.peers.values() {
            peer.send_message(packet.clone()).await;
        }
    }
    async fn cycle(&mut self) {
        select! {
            Some(message) = self.message_receiver.recv() => {
                self.handle_message(message).await;
            },
            Some(packet) = self.packet_receiver.recv() => {
                self.handle_packet(packet).await;
            }
        };
    }
    pub async fn run(mut self) {
        loop {
            self.cycle().await;
        }
    }
}

impl Actor<PeerCollectionMessage> for PeerCollection {
    fn get_addr(&self) -> super::Addr<PeerCollectionMessage> {
        self.message_address.clone()
    }
}

impl Actor<Packet> for PeerCollection {
    fn get_addr(&self) -> Addr<Packet> {
        self.packet_address.clone()
    }
}
