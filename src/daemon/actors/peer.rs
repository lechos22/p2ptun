use std::sync::Arc;

use quinn::{Connection, RecvStream, SendStream};
use tokio::{select, sync::mpsc};

use crate::daemon::packet::Packet;

use super::{Actor, Addr};

pub struct Peer {
    packet_address: Addr<Packet>,
    packet_receiver: mpsc::Receiver<Packet>,
    peer_collection: Addr<Packet>,
    connection: Connection,
}

impl Peer {
    pub fn new(peer_collection: Addr<Packet>, connection: Connection) -> Self {
        let (packet_sender, packet_receiver) = mpsc::channel(16);
        Self {
            packet_address: Addr::new(packet_sender),
            packet_receiver,
            peer_collection,
            connection,
        }
    }
    async fn send_packets(mut recv_stream: RecvStream, peer_collection: Addr<Packet>) {
        let mut buffer = vec![0u8; 1518];
        while let Ok(Some(size)) = recv_stream.read(&mut buffer).await {
            peer_collection
                .send_message(Packet::Incoming(Arc::from(&buffer[0..size])))
                .await;
        }
    }
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
    pub async fn run(self) {
        let Ok(send_stream) = self.connection.open_uni().await else {
            return;
        };
        let Ok(recv_stream) = self.connection.accept_uni().await else {
            return;
        };
        select! {
            _ = Self::send_packets(recv_stream, self.peer_collection) => {}
            _ = Self::recv_packets(send_stream, self.packet_receiver) => {}
        }
    }
}

impl Actor<Packet> for Peer {
    fn get_addr(&self) -> Addr<Packet> {
        self.packet_address.clone()
    }
}
