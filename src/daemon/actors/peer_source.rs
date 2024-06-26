//! Module for [PeerSource] actor.
//!
//! It is responsible for acquiring connections with other peers.

use iroh_net::{
    key::SecretKey, magic_endpoint::accept_conn, relay::RelayMode, ticket::NodeTicket,
    MagicEndpoint, NodeAddr, NodeId,
};
use quinn::Connection;
use tokio::sync::mpsc;

use crate::daemon::{packet::Packet, DaemonError};

use super::{peer::Peer, peer_collection::PeerCollectionMessage, Actor, Addr};

/// Messages that can be sent to [PeerSource].
#[derive(Debug, Clone)]
pub enum PeerSourceMessage {
    /// Instructs [PeerSource] to initiate a connection with the specified [NodeAddr].
    DialPeer(NodeAddr),
}

/// Creates a future from a closure returning an option.
async fn future_option<T>(f: impl Fn() -> Option<T>) -> T {
    loop {
        if let Some(value) = f() {
            return value;
        }
        tokio::task::yield_now().await;
    }
}

const ALPN: &[u8] = "p2ptun".as_bytes();

/// Represents the mode of establishing streams on [Connection].
enum ChannelMode {
    Accept,
    Open,
}

/// An actor that initiates and accepts connections to peers using [MagicEndpoint].
pub struct PeerSource {
    address: Addr<PeerSourceMessage>,
    receiver: mpsc::Receiver<PeerSourceMessage>,
    peers_message_addr: Addr<PeerCollectionMessage>,
    peers_packet_addr: Addr<Packet>,
    magic_endpoint: MagicEndpoint,
}
impl PeerSource {
    /// Creates a new [PeerSource] actor.
    pub async fn new<PeerCollectionActor>(
        peer_collection: &PeerCollectionActor,
        secret_key: SecretKey,
    ) -> Result<Self, DaemonError>
    where
        PeerCollectionActor: Actor<PeerCollectionMessage> + Actor<Packet>,
    {
        // Create the magic endpoint
        let magic_endpoint = MagicEndpoint::builder()
            .alpns(vec![ALPN.to_vec()])
            .relay_mode(RelayMode::Default)
            .secret_key(secret_key)
            .bind(0)
            .await?;
        // Wait for connection to a relay
        // TODO: Handle the case when connection to a relay can't succeed
        future_option(|| magic_endpoint.my_relay()).await;
        // Create the message channel
        let (sender, receiver) = mpsc::channel(16);
        // Pack the struct
        Ok(Self {
            address: Addr::new(sender),
            receiver,
            peers_message_addr: peer_collection.get_addr(),
            peers_packet_addr: peer_collection.get_addr(),
            magic_endpoint,
        })
    }
    /// Retrieves the [NodeTicket] for this [PeerSource].
    pub async fn node_ticket(&self) -> Result<NodeTicket, DaemonError> {
        let node_addr = self.magic_endpoint.my_addr().await?;
        Ok(NodeTicket::new(node_addr)?)
    }
    /// Handles incoming messages to [PeerSource].
    async fn handle_messages(
        peers_packet_addr: &Addr<Packet>,
        peers_message_addr: &Addr<PeerCollectionMessage>,
        receiver: &mut mpsc::Receiver<PeerSourceMessage>,
        magic_endpoint: &MagicEndpoint,
    ) {
        loop {
            let message = match receiver.recv().await {
                Some(message) => message,
                None => continue,
            };
            match message {
                PeerSourceMessage::DialPeer(node_addr) => {
                    tokio::spawn(Self::dial_peer(
                        peers_packet_addr.clone(),
                        peers_message_addr.clone(),
                        node_addr,
                        magic_endpoint.clone(),
                    ));
                }
            }
        }
    }
    /// Handles incoming connections from [MagicEndpoint].
    async fn handle_connections(
        peers_packet_addr: &Addr<Packet>,
        peers_message_addr: &Addr<PeerCollectionMessage>,
        magic_endpoint: &MagicEndpoint,
    ) {
        while let Some(connecting) = magic_endpoint.accept().await {
            tokio::spawn(Self::handle_connecting(
                connecting,
                peers_packet_addr.clone(),
                peers_message_addr.clone(),
            ));
        }
    }
    /// Handles one incoming connection.
    async fn handle_connecting(
        connecting: quinn::Connecting,
        peers_packet_addr: Addr<Packet>,
        peers_message_addr: Addr<PeerCollectionMessage>,
    ) {
        if let Ok((node_id, _, connection)) = accept_conn(connecting).await {
            // TODO: Check if the connection should be blocked
            Self::handle_connection(
                node_id,
                connection,
                peers_packet_addr.clone(),
                peers_message_addr.clone(),
                ChannelMode::Accept,
            )
            .await;
        }
    }
    /// Creates a connection to the peer with the specified [NodeAddr].
    async fn dial_peer(
        peers_packet_addr: Addr<Packet>,
        peers_message_addr: Addr<PeerCollectionMessage>,
        node_addr: NodeAddr,
        magic_endpoint: MagicEndpoint,
    ) {
        match magic_endpoint.connect(node_addr.clone(), ALPN).await {
            Ok(connection) => {
                tokio::spawn(Self::handle_connection(
                    node_addr.node_id,
                    connection,
                    peers_packet_addr,
                    peers_message_addr,
                    ChannelMode::Open,
                ));
            }
            Err(error) => {
                eprintln!(
                    "Couldn't dial the peer {}. Reason: {:?}",
                    node_addr.node_id, error
                );
            }
        }
    }
    /// Handles an established connection to a peer by opening streams on the connection and registers the peer.
    async fn handle_connection(
        node_id: NodeId,
        connection: Connection,
        peers_packet_addr: Addr<Packet>,
        peers_message_addr: Addr<PeerCollectionMessage>,
        channel_mode: ChannelMode,
    ) {
        let streams = match channel_mode {
            ChannelMode::Accept => connection.accept_bi().await,
            ChannelMode::Open => connection.open_bi().await,
        };
        let (send_stream, recv_stream) = match streams {
            Ok(streams) => streams,
            Err(error) => {
                eprintln!(
                    "Error establishing streams with {}, Reason: {:?}",
                    node_id, error
                );
                return;
            }
        };
        let peer = Peer::new(peers_packet_addr, send_stream, recv_stream);
        peers_message_addr
            .send_message(PeerCollectionMessage::AddPeer(node_id, peer))
            .await;
    }
    /// Runs the actor.
    pub async fn run(mut self) {
        tokio::select! {
            _ = Self::handle_messages(&self.peers_packet_addr, &self.peers_message_addr, &mut self.receiver, &self.magic_endpoint) => {}
            _ = Self::handle_connections(&self.peers_packet_addr, &self.peers_message_addr, &self.magic_endpoint) => {}
        }
    }
}
impl Actor<PeerSourceMessage> for PeerSource {
    fn get_addr(&self) -> super::Addr<PeerSourceMessage> {
        self.address.clone()
    }
}
