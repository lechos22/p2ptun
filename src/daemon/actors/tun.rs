use std::{sync::Arc, time::Duration};

use tokio::{select, sync::mpsc};

use crate::daemon::packet::Packet;

use super::{Actor, Addr};

pub struct Tun {
    address: Addr<Packet>,
    packet_router: Addr<Packet>,
    receiver: mpsc::Receiver<Packet>,
}

impl Tun {
    pub fn new(packet_router: Addr<Packet>) -> Self {
        let (sender, receiver) = mpsc::channel(16);
        Self {
            address: Addr::new(sender),
            receiver,
            packet_router,
        }
    }
    async fn send_packets(packet_router: Addr<Packet>) {
        loop {
            todo!()
        }
    }
    async fn recv_packets(mut receiver: mpsc::Receiver<Packet>) {
        loop {
            let Some(Packet::Incoming(packet)) = receiver.recv().await else {
                continue;
            };
            todo!()
        }
    }
    pub async fn run(self) {
        select! {
            _ = Self::send_packets(self.packet_router) => {}
            _ = Self::recv_packets(self.receiver) => {}
        }
    }
}

impl Actor<Packet> for Tun {
    fn get_addr(&self) -> super::Addr<Packet> {
        return self.address.clone();
    }
}
