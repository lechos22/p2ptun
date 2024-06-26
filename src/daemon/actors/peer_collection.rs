//! Module for [PeerCollection] actor.
//!
//! It is responsible for managing connected peers.

use std::collections::HashMap;

use iroh_net::NodeId;
use tokio::{select, sync::mpsc, task::AbortHandle};

use crate::daemon::packet::Packet;

use super::{peer::Peer, Actor, Addr};

/// Messages that can be sent to [PeerCollection].
pub enum PeerCollectionMessage {
    /// Instructs [PeerCollection] to add a peer with the specified [NodeId] and [Peer] instance.
    AddPeer(NodeId, Peer),
    /// Instructs [PeerCollection] to remove a peer identified by the given [NodeId].
    DisconnectPeer(NodeId),
}
struct PeerWrapper {
    abort_handle: AbortHandle,
    address: Addr<Packet>,
}
/// Manages a collection of peers and handles peer-related messages and packet routing.
pub struct PeerCollection {
    message_address: Addr<PeerCollectionMessage>,
    message_receiver: mpsc::Receiver<PeerCollectionMessage>,
    router_address: Addr<Packet>,
    packet_address: Addr<Packet>,
    packet_receiver: mpsc::Receiver<Packet>,
    peers: HashMap<NodeId, PeerWrapper>,
}
impl PeerCollection {
    /// Creates a new instance with the specified `router_address`.
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
    /// Handles a received message.
    async fn handle_message(&mut self, message: PeerCollectionMessage) {
        match message {
            PeerCollectionMessage::AddPeer(node_id, peer) => {
                self.add_peer(node_id, peer);
            }
            PeerCollectionMessage::DisconnectPeer(node_id) => {
                self.disconnect_peer(node_id);
            }
        }
    }
    /// Adds a peer to the collection identified by the provided [NodeId].
    fn add_peer(&mut self, node_id: NodeId, peer: Peer) {
        println!("Connected to peer {}", node_id);
        let peer_addr = peer.get_addr();
        let message_address = self.message_address.clone();
        let join_handle = tokio::spawn(async move {
            peer.run().await;
            message_address
                .send_message(PeerCollectionMessage::DisconnectPeer(node_id))
                .await;
        });
        self.peers.insert(
            node_id,
            PeerWrapper {
                abort_handle: join_handle.abort_handle(),
                address: peer_addr,
            },
        );
    }
    /// Disconnects from a peer identified by the provided [NodeId].
    fn disconnect_peer(&mut self, node_id: NodeId) {
        if let Some(peer) = self.peers.remove(&node_id) {
            println!("Disconnected from peer {}", node_id);
            peer.abort_handle.abort();
        }
    }
    /// Handles a received packet.
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
    /// Sends a packet to all connected peers in the collection.
    async fn send_packet_to_peers(&self, packet: &Packet) {
        for peer in self.peers.values() {
            peer.address.send_message(packet.clone()).await;
        }
    }
    /// Executes a single cycle of message handling or packet processing.
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
    /// Runs the actor, continuously processing messages and packets.
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
