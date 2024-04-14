//! The p2ptun's daemon. It is responsible for the most of the program's functionality.

pub mod channel;

use std::time::Duration;

use tokio::{
    sync::broadcast::{self, Sender},
    task::JoinSet,
};

use self::channel::{
    log_channel::LogChannelManager, tun_channel::TunChannelManager, ChannelManager, Packet,
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
    /// Spawns a job for handling some channel.
    fn spawn_channel_job(&mut self, channel_manager: impl ChannelManager + Send + Sync + 'static) {
        let job = channel_manager.run(self.packet_sender.clone());
        self.channel_jobs.spawn(job);
    }
    /// Runs the daemon.
    pub async fn run(mut self) {
        self.spawn_channel_job(LogChannelManager);
        self.spawn_channel_job(TunChannelManager);

        // Await for Ctrl+C
        let _ = tokio::signal::ctrl_c().await;
    }
}
