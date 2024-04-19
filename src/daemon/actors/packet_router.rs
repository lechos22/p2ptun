//! Module for [PacketRouter] actor.
//!
//! It is responsible for sending packet to right actors.

use tokio::sync::mpsc;

use crate::daemon::packet::Packet;

use super::{Actor, Addr};

/// Represents a packet routing actor responsible for distributing packets to multiple receivers.
pub struct PacketRouter {
    /// The receiver channel for incoming packets.
    packet_receiver: mpsc::Receiver<Packet>,

    /// The address used to send packets to this router.
    address: Addr<Packet>,

    /// Collection of addresses of incoming packet receivers connected to this router.
    incoming_packet_receivers: Vec<Addr<Packet>>,

    /// Collection of addresses of outgoing packet receivers connected to this router.
    outgoing_packet_receivers: Vec<Addr<Packet>>,
}

impl PacketRouter {
    /// Creates a new [PacketRouter] instance.
    ///
    /// Returns a [PacketRouter] with its associated receiver channel for incoming packets,
    /// an [Addr] for sending packets to this router, and an empty list of packet receivers.
    pub fn new() -> Self {
        let (packet_sender, packet_receiver) = mpsc::channel(16);
        Self {
            packet_receiver,
            address: Addr::new(packet_sender),
            incoming_packet_receivers: Vec::new(),
            outgoing_packet_receivers: Vec::new(),
        }
    }

    /// Adds a new incoming packet receiver to this router.
    ///
    /// Parameters:
    /// - `addr`: The address of the packet receiver to add.
    pub fn add_incoming_packet_receiver(&mut self, addr: Addr<Packet>) {
        self.incoming_packet_receivers.push(addr);
    }

    /// Adds a new outgoing packet receiver to this router.
    ///
    /// Parameters:
    /// - `addr`: The address of the packet receiver to add.
    pub fn add_outgoing_packet_receiver(&mut self, addr: Addr<Packet>) {
        self.outgoing_packet_receivers.push(addr);
    }

    /// Runs the packet router asynchronously.
    ///
    /// This method continuously receives packets from the receiver channel
    /// and forwards each received packet to all connected packet receivers.
    pub async fn run(mut self) {
        loop {
            // Attempt to receive a packet from the receiver channel.
            let packet = match self.packet_receiver.recv().await {
                Some(packet) => packet,
                None => continue, // If receive fails, continue to the next iteration.
            };

            match packet {
                packet @ Packet::Outgoing(_) => {
                    // Send the received packet to each connected outgoing packet receiver.
                    for addr in &self.outgoing_packet_receivers {
                        addr.send_message(packet.clone()).await;
                    }
                }
                packet @ Packet::Incoming(_) => {
                    // Send the received packet to each connected incoming packet receiver.
                    for addr in &self.incoming_packet_receivers {
                        addr.send_message(packet.clone()).await;
                    }
                }
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
