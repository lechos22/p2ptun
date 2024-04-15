pub mod log_channel;
pub mod noise_channel;
pub mod quic_channel;

use std::{future::Future, pin::Pin, sync::Arc};

use tokio::{sync::broadcast, task::JoinSet};

/// A packet of data.
#[derive(Clone)]
pub enum Packet {
    /// Represents an incoming packet with its data.
    Incoming(Arc<[u8]>),
    /// Represents an outgoing packet with its data.
    Outgoing(Arc<[u8]>),
}

/// Custom error type for channel operations.
pub enum ChannelError {
    Closed,
    Io(std::io::Error),
}

/// A future type representing the result of a channel operation.
pub type ChannelJobFuture = Pin<Box<dyn Future<Output = Result<(), ChannelError>> + Send>>;

/// The trait responsible for managing a channel.
pub trait ChannelManager {
    /// Opens a channel and returns a future resolving to a managed channel instance.
    fn open(&self) -> impl Future<Output = Arc<dyn Channel>> + Send;

    /// Runs the channel manager, continuously handling packet publishing and subscribing.
    fn run(
        self,
        packet_sender: broadcast::Sender<Packet>,
    ) -> Pin<Box<dyn Future<Output = !> + Send>>
    where
        Self: Sized + Sync + Send + 'static,
    {
        Box::pin(async move {
            let mut channel = self.open().await;
            loop {
                let mut join_set = JoinSet::new();
                join_set.spawn(channel.publish_packets(packet_sender.clone()));
                join_set.spawn(channel.subscribe_packets(packet_sender.subscribe()));
                let _ = join_set.join_next().await;
                channel = self.open().await;
            }
        })
    }
}

/// A trait defining channels' behavior.
pub trait Channel: Sync + Send {
    /// Subscribes to packets from a broadcast receiver.
    fn subscribe_packets(&self, packet_receiver: broadcast::Receiver<Packet>) -> ChannelJobFuture;

    /// Publishes packets through a broadcast sender.
    fn publish_packets(&self, packet_sender: broadcast::Sender<Packet>) -> ChannelJobFuture;
}
