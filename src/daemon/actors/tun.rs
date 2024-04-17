//! Module for TUN (network tunnel) actor.

use std::sync::Arc;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    select,
    sync::mpsc,
};
use tun::{configure, AsyncDevice};

use crate::daemon::packet::Packet;

use super::{Actor, Addr};

/// Represents a TUN (network tunnel) actor for handling network traffic.
pub struct Tun {
    /// The address used to send packets to the TUN actor.
    address: Addr<Packet>,

    /// The address of the packet router to forward packets to.
    packet_router: Addr<Packet>,

    /// The receiver channel for incoming packets to be sent via the TUN device.
    receiver: mpsc::Receiver<Packet>,

    /// The TUN device used for reading and writing network packets.
    tun: AsyncDevice,
}

impl Tun {
    /// Creates a new [Tun] instance.
    ///
    /// Parameters:
    /// - `packet_router`: The address of the packet router to forward packets to.
    ///
    /// Returns a [Tun] instance with its associated receiver channel and TUN device.
    pub fn new(packet_router: Addr<Packet>) -> tun::Result<Self> {
        let (sender, receiver) = mpsc::channel(16);
        Ok(Self {
            address: Addr::new(sender),
            receiver,
            packet_router,
            tun: tun::create_as_async(configure().up())?, // Create the TUN device
        })
    }

    /// Asynchronously sends packets received from the TUN device to the packet router.
    async fn send_packets(mut tun_read: ReadHalf<AsyncDevice>, packet_router: Addr<Packet>) {
        loop {
            let mut buffer = vec![0u8; 1518];
            match tun_read.read(&mut buffer).await {
                Ok(size) if size > 0 => {
                    // Send the outgoing packet to the packet router
                    packet_router
                        .send_message(Packet::Outgoing(Arc::from(&buffer[0..size])))
                        .await;
                }
                _ => continue, // Ignore empty or error packets
            }
        }
    }

    /// Asynchronously receives packets from the packet receiver and writes them to the TUN device.
    async fn recv_packets(mut tun_write: WriteHalf<AsyncDevice>, mut receiver: mpsc::Receiver<Packet>) {
        loop {
            if let Some(Packet::Incoming(packet)) = receiver.recv().await {
                // Write the incoming packet to the TUN device
                let _ = tun_write.write(&packet).await;
            } else {
                continue; // Ignore non-incoming packets
            }
        }
    }

    /// Runs the TUN actor asynchronously, handling packet I/O operations.
    pub async fn run(self) {
        let (tun_read, tun_write) = tokio::io::split(self.tun);

        // Use `select!` to concurrently handle packet sending and receiving
        select! {
            _ = Self::send_packets(tun_read, self.packet_router) => {} // Handle packet sending
            _ = Self::recv_packets(tun_write, self.receiver) => {} // Handle packet receiving
        }
    }
}

impl Actor<Packet> for Tun {
    fn get_addr(&self) -> super::Addr<Packet> {
        return self.address.clone();
    }
}
