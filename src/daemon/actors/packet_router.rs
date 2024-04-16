use tokio::sync::mpsc::{self, Receiver};

use crate::daemon::packet::Packet;

use super::{Actor, Addr};

pub struct PacketRouter {
    packet_receiver: Receiver<Packet>,
    address: Addr<Packet>,
    packet_receivers: Vec<Addr<Packet>>,
}

impl PacketRouter {
    pub fn new() -> Self {
        let (packet_sender, packet_receiver) = mpsc::channel(16);
        Self {
            packet_receiver,
            address: Addr::new(packet_sender),
            packet_receivers: Vec::new(),
        }
    }
    pub fn add_packet_receiver(&mut self, addr: Addr<Packet>) {
        self.packet_receivers.push(addr);
    }
    pub async fn run(mut self) {
        loop {
            let Some(packet) = self.packet_receiver.recv().await else {
                continue;
            };
            for addr in &self.packet_receivers {
                addr.send_message(packet.clone()).await;
            }
        }
    }
}

impl Default for PacketRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl Actor<Packet> for PacketRouter {
    fn get_addr(&self) -> Addr<Packet> {
        self.address.clone()
    }
}
