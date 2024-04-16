use std::sync::Arc;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    select,
    sync::mpsc,
};
use tun::{configure, AsyncDevice};

use crate::daemon::packet::Packet;

use super::{Actor, Addr};

pub struct Tun {
    address: Addr<Packet>,
    packet_router: Addr<Packet>,
    receiver: mpsc::Receiver<Packet>,
    tun: AsyncDevice,
}

impl Tun {
    pub fn new(packet_router: Addr<Packet>) -> tun::Result<Self> {
        let (sender, receiver) = mpsc::channel(16);
        Ok(Self {
            address: Addr::new(sender),
            receiver,
            packet_router,
            tun: tun::create_as_async(configure().up())?,
        })
    }
    async fn send_packets(mut tun_read: ReadHalf<AsyncDevice>, packet_router: Addr<Packet>) {
        loop {
            let mut buffer = vec![0u8; 1518];
            let Ok(size) = tun_read.read(&mut buffer).await else {
                continue;
            };
            packet_router.send_message(Packet::Outgoing(Arc::from(&buffer[0..size]))).await;
        }
    }
    async fn recv_packets(
        mut tun_write: WriteHalf<AsyncDevice>,
        mut receiver: mpsc::Receiver<Packet>,
    ) {
        loop {
            let Some(Packet::Incoming(packet)) = receiver.recv().await else {
                continue;
            };
            let _ = tun_write.write(&packet).await;
        }
    }
    pub async fn run(self) {
        let (tun_read, tun_write) = tokio::io::split(self.tun);
        select! {
            _ = Self::send_packets(tun_read, self.packet_router) => {}
            _ = Self::recv_packets(tun_write, self.receiver) => {}
        }
    }
}

impl Actor<Packet> for Tun {
    fn get_addr(&self) -> super::Addr<Packet> {
        return self.address.clone();
    }
}
