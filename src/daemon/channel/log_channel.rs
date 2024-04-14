//! A channel that logs all packets. Might be useful for debugging.

use std::sync::Arc;

use tokio::sync::broadcast;

use crate::daemon::sleep_forever;

use super::{Channel, ChannelJobFuture, ChannelManager, Packet};

pub struct LogChannel;

impl Channel for LogChannel {
    fn subscribe_packets(
        &self,
        mut packet_receiver: broadcast::Receiver<Packet>,
    ) -> ChannelJobFuture {
        Box::pin(async move {
            loop {
                let packet = packet_receiver.recv().await.unwrap();
                match packet {
                    Packet::Incoming(content) => {
                        eprintln!("A packet with size {} was received.", content.len());
                    }
                    Packet::Outgoing(content) => {
                        eprintln!("A packet with size {} was sent.", content.len());
                    }
                }
            }
        })
    }

    fn publish_packets(&self, _: broadcast::Sender<Packet>) -> ChannelJobFuture {
        Box::pin(sleep_forever())
    }
}

pub struct LogChannelManager;
impl ChannelManager for LogChannelManager {
    async fn open(&self) -> Arc<dyn Channel> {
        Arc::new(LogChannel)
    }
}
