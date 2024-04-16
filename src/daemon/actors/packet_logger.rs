use tokio::sync::mpsc;

use crate::daemon::packet::Packet;

use super::{Actor, Addr};

pub struct PacketLogger {
    address: Addr<Packet>,
    receiver: mpsc::Receiver<Packet>,
}

impl PacketLogger {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(16);
        Self {
            address: Addr::new(sender),
            receiver,
        }
    }
    pub async fn run(mut self) {
        loop {
            let Some(packet) = self.receiver.recv().await else {
                continue;
            };
            eprintln!("{:?}", packet);
        }
    }
}

impl Default for PacketLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl Actor<Packet> for PacketLogger {
    fn get_addr(&self) -> super::Addr<Packet> {
        return self.address.clone();
    }
}
