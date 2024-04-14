//! The p2ptun's daemon. It is responsible for the most of the program's functionality.

pub mod channel;

use std::time::Duration;

use tokio::{
    sync::broadcast::{self, Sender},
    task::JoinSet,
};

use self::channel::{
    log_channel::LogChannelManager, noise_channel::NoiseChannelManager, ChannelManager, Packet,
};

/// An async function that sleeps forever.
async fn sleep_forever<T>() -> T {
    tokio::time::sleep(Duration::MAX).await;
    unreachable!()
}

/// The p2ptun's daemon configuration
#[derive(Default)]
pub struct DaemonConfig {}

/// The p2ptun's daemon
pub struct Daemon {
    packet_sender: Sender<Packet>,
    channel_jobs: JoinSet<!>,
}

impl Daemon {
    /// Creates the daemon.
    pub fn new(_config: DaemonConfig) -> Self {
        Self {
            packet_sender: broadcast::channel(64).0,
            channel_jobs: JoinSet::new(),
        }
    }
    /// Runs the daemon.
    pub async fn run(mut self) {
        self.channel_jobs
            .spawn(LogChannelManager.run(self.packet_sender.clone()));
        self.channel_jobs
            .spawn(NoiseChannelManager.run(self.packet_sender.clone()));

        // Await for Ctrl+C
        let _ = tokio::signal::ctrl_c().await;
    }
}
