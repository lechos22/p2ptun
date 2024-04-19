//! Module for [Peer] actor.
//! 
//! It is responsible for transmitting data to and from a peer.

use std::sync::Arc;

use quinn::{RecvStream, SendStream};
use tokio::{select, sync::mpsc};

use crate::daemon::packet::Packet;

use super::{Actor, Addr};

/// Represents a peer actor responsible for transmitting data to and from a peer.
pub struct Peer {
    packet_address: Addr<Packet>,
    packet_receiver: mpsc::Receiver<Packet>,
    peer_collection: Addr<Packet>,
    send_stream: SendStream,
    recv_stream: RecvStream,
}

impl Peer {
    /// Creates a new instance with the given parameters.
    pub fn new(
        peer_collection: Addr<Packet>,
        send_stream: SendStream,
        recv_stream: RecvStream,
    ) -> Self {
        let (packet_sender, packet_receiver) = mpsc::channel(16);
        Self {
            packet_address: Addr::new(packet_sender),
            packet_receiver,
            peer_collection,
            send_stream,
            recv_stream,
        }
    }
    /// Sends received packets from the peer's receive stream to the peer collection.
    async fn send_packets(mut recv_stream: RecvStream, peer_collection: Addr<Packet>) {
        let mut buffer = vec![0u8; 1518];
        while let Ok(Some(size)) = recv_stream.read(&mut buffer).await {
            peer_collection
                .send_message(Packet::Incoming(Arc::from(&buffer[0..size])))
                .await;
        }
    }
    /// Receives outgoing packets from the peer collection and sends them via the send stream.
    async fn recv_packets(
        mut send_stream: SendStream,
        mut packet_receiver: mpsc::Receiver<Packet>,
    ) {
        loop {
            if let Some(Packet::Outgoing(packet)) = packet_receiver.recv().await {
                if send_stream.write(&packet).await.is_err() {
                    return;
                }
            } else {
                continue;
            }
        }
    }
    /// Runs the actor, handling send and receive operations concurrently.
    pub async fn run(self) {
        select! {
            _ = Self::send_packets(self.recv_stream, self.peer_collection) => {}
            _ = Self::recv_packets(self.send_stream, self.packet_receiver) => {}
        }
    }
}

impl Actor<Packet> for Peer {
    fn get_addr(&self) -> Addr<Packet> {
        self.packet_address.clone()
    }
}
