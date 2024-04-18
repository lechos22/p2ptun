use iroh_net::{
    key::SecretKey, relay::RelayMode, ticket::NodeTicket, tls::certificate::P2pCertificate,
    MagicEndpoint, NodeAddr,
};
use quinn::Connection;
use tokio::sync::mpsc;

use crate::daemon::{packet::Packet, DaemonError};

use super::{peer::Peer, peer_collection::PeerCollectionMessage, Actor, Addr};

#[derive(Debug, Clone)]
pub enum PeerSourceMessage {
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

/// An actor that initiates and accepts connections to peers.
pub struct PeerSource {
    address: Addr<PeerSourceMessage>,
    receiver: mpsc::Receiver<PeerSourceMessage>,
    peers_message_addr: Addr<PeerCollectionMessage>,
    peers_packet_addr: Addr<Packet>,
    magic_endpoint: MagicEndpoint,
}
impl PeerSource {
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
    pub async fn node_ticket(&self) -> Result<NodeTicket, DaemonError> {
        let node_addr = self.magic_endpoint.my_addr().await?;
        Ok(NodeTicket::new(node_addr)?)
    }
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
    async fn handle_connections(
        peers_packet_addr: &Addr<Packet>,
        peers_message_addr: &Addr<PeerCollectionMessage>,
        magic_endpoint: &MagicEndpoint,
    ) {
        while let Some(connecting) = magic_endpoint.accept().await {
            // TODO: Check if the connection should be blocked
            if let Ok(connection) = connecting.await {
                tokio::spawn(Self::handle_connection(
                    connection,
                    peers_packet_addr.clone(),
                    peers_message_addr.clone(),
                ));
            }
        }
    }
    async fn dial_peer(
        peers_packet_addr: Addr<Packet>,
        peers_message_addr: Addr<PeerCollectionMessage>,
        node_addr: NodeAddr,
        magic_endpoint: MagicEndpoint,
    ) {
        match magic_endpoint.connect(node_addr.clone(), ALPN).await {
            Ok(connection) => {
                tokio::spawn(Self::handle_connection(
                    connection,
                    peers_packet_addr,
                    peers_message_addr,
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
    async fn handle_connection(
        connection: Connection,
        peers_packet_addr: Addr<Packet>,
        peers_message_addr: Addr<PeerCollectionMessage>,
    ) {
        let Some(peer_identity) = connection.peer_identity() else {
            eprintln!("Couldn't retreive peer's certificate.");
            return;
        };
        let cert = match peer_identity.downcast::<P2pCertificate>() {
            Ok(cert) => cert,
            Err(error) => {
                eprintln!("Couldn't downcast peer's certificate. Reason: {:?}", error);
                return;
            }
        };
        let node_id = cert.peer_id();
        let peer = Peer::new(peers_packet_addr, connection);
        peers_message_addr
            .send_message(PeerCollectionMessage::AddPeer(node_id, peer))
            .await;
    }
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
