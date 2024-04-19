//! Module for defining network packet types used in the VPN tunnel.
//!
//! This module defines the [Packet] enum, which represents different types of network packets
//! used within the VPN tunnel implementation.
//!
//! The [Packet] enum is designed to facilitate packet handling and routing within the VPN
//! tunnel, providing a standardized representation for network traffic.

use std::{fmt::Debug, sync::Arc};

/// Represents a network packet used in the VPN tunnel.
#[derive(Clone)]
pub enum Packet {
    /// Outgoing packet containing data to be transmitted.
    Outgoing(Arc<[u8]>),

    /// Incoming packet containing received data.
    Incoming(Arc<[u8]>),
}

impl Debug for Packet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Outgoing(arg0) => f.debug_tuple("Outgoing").field(&arg0.len()).finish(),
            Self::Incoming(arg0) => f.debug_tuple("Incoming").field(&arg0.len()).finish(),
        }
    }
}
