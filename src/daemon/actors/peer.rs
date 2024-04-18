use std::sync::Arc;

use quinn::{RecvStream, SendStream};
use tokio::{select, sync::mpsc};

use crate::daemon::packet::Packet;

use super::{Actor, Addr};

pub struct Peer {
    packet_address: Addr<Packet>,
    packet_receiver: mpsc::Receiver<Packet>,
    peer_collection: Addr<Packet>,
    send_stream: SendStream,
    recv_stream: RecvStream,
}

impl Peer {
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
