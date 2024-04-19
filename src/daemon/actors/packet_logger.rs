//! Module for [PacketLogger] actor.
//!
//! It is responsible for keeping a log of packets going through the program.

use tokio::sync::mpsc;

use crate::daemon::packet::Packet;

use super::{Actor, Addr};

/// Represents a packet logger actor responsible for logging all packets.
pub struct PacketLogger {
    /// The address used to send packets to this logger.
    address: Addr<Packet>,

    /// The receiver channel for incoming packets.
    receiver: mpsc::Receiver<Packet>,
}

impl PacketLogger {
    /// Creates a new [PacketLogger] instance.
    ///
    /// Returns a [PacketLogger] with its associated [Addr] for sending packets
    /// and a receiver channel for receiving packets.
    pub fn new() -> Self {
        // Create a channel for communication between sender and receiver.
        let (sender, receiver) = mpsc::channel(16);
        // Create a new `Addr` using the sender channel.
        let address = Addr::new(sender);
        Self { address, receiver }
    }

    /// Runs the packet logger asynchronously.
    ///
    /// This method continuously receives packets from the receiver channel
    /// and logs each received packet to standard error ([eprintln]).
    pub async fn run(mut self) {
        loop {
            // Attempt to receive a packet from the receiver channel.
            let packet = match self.receiver.recv().await {
                Some(packet) => packet,
                None => continue, // If receive fails, continue to the next iteration.
            };

            // Log the received packet to standard error.
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
