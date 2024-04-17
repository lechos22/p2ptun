use tokio::sync::mpsc;

use crate::daemon::packet::Packet;

use super::{Actor, Addr};

pub struct Peer {
    packet_address: Addr<Packet>,
    packet_receiver: mpsc::Receiver<Packet>,
    peer_collection: Addr<Packet>,
}

impl Peer {
    pub fn new(peer_collection: Addr<Packet>) -> Self {
        let (packet_sender, packet_receiver) = mpsc::channel(16);
        Self {
            packet_address: Addr::new(packet_sender),
            packet_receiver,
            peer_collection,
        }
    }
    pub async fn run(mut self) {
        loop {
            todo!();
        }
    }
}

impl Actor<Packet> for Peer {
    fn get_addr(&self) -> Addr<Packet> {
        self.packet_address.clone()
    }
}
