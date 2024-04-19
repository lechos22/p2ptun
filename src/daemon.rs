//! The p2ptun's daemon. It is responsible for the most of the program's functionality.

pub mod actors;
pub mod packet;

use iroh_net::key::SecretKey;
use tokio::{select, task::JoinSet};

use crate::daemon::actors::{
    packet_logger::PacketLogger, packet_router::PacketRouter, peer_collection::PeerCollection,
    peer_source::PeerSource, tun::Tun, Actor,
};

/// The p2ptun's daemon configuration
#[derive(Default)]
pub struct DaemonConfig {
    pub enable_tun: bool,
}

/// Enum representing errors that can happen in p2ptun's daemon
#[derive(Debug)]
pub enum DaemonError {
    TunError(tun::Error),
    AnyhowError(anyhow::Error),
    Died,
}

impl From<tun::Error> for DaemonError {
    fn from(error: tun::Error) -> Self {
        Self::TunError(error)
    }
}

impl From<anyhow::Error> for DaemonError {
    fn from(error: anyhow::Error) -> Self {
        Self::AnyhowError(error)
    }
}

/// The p2ptun's daemon
pub async fn run_daemon(config: DaemonConfig) -> Result<(), DaemonError> {
    // Create the secret key
    let secret_key = SecretKey::generate();
    println!("Node ID: {}", secret_key.public());

    // Initialize actors
    let mut packet_router = PacketRouter::new();
    let packet_logger = PacketLogger::new();
    let peer_collection = PeerCollection::new(packet_router.get_addr());
    let peer_source = PeerSource::new(&peer_collection, secret_key).await?;
    println!("Node ticket: {}", peer_source.node_ticket().await?);
    let tun = if config.enable_tun {
        let tun = Tun::new(packet_router.get_addr())?;
        packet_router.add_incoming_packet_receiver(tun.get_addr());
        Some(tun)
    } else {
        None
    };
    packet_router.add_incoming_packet_receiver(packet_logger.get_addr());
    packet_router.add_outgoing_packet_receiver(packet_logger.get_addr());
    packet_router.add_outgoing_packet_receiver(peer_collection.get_addr());

    // Run
    let mut join_set = JoinSet::new();
    join_set.spawn(packet_logger.run());
    join_set.spawn(packet_router.run());
    join_set.spawn(peer_collection.run());
    join_set.spawn(peer_source.run());
    if let Some(tun) = tun {
        join_set.spawn(tun.run());
    }
    select! {
        _ = join_set.join_next() => {Err(DaemonError::Died)}
        _ = tokio::signal::ctrl_c() => {
            println!("\nStopping...");
            Ok(())
        }
    }
}
