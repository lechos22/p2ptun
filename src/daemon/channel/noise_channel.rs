//! A channel that creates random packets. It's only meant
//! for testing the p2ptun's architecture.

use std::{sync::Arc, time::Duration};

use tokio::sync::broadcast;

use crate::daemon::sleep_forever;

use super::{Channel, ChannelJobFuture, ChannelManager, Packet};

pub struct NoiseChannel;
impl Channel for NoiseChannel {
    fn subscribe_packets(&self, _: broadcast::Receiver<Packet>) -> ChannelJobFuture {
        Box::pin(sleep_forever())
    }

    fn publish_packets(&self, packet_sender: broadcast::Sender<Packet>) -> ChannelJobFuture {
        extern "C" {
            fn rand() -> std::ffi::c_int;
        }
        fn rand_mtu() -> usize {
            (unsafe { rand() } % 1500) as usize
        }
        fn rand_u8() -> u8 {
            (unsafe { rand() } % 256) as u8
        }
        fn rand_bool() -> bool {
            (unsafe { rand() } % 2) == 0
        }
        Box::pin(async move {
            loop {
                let mut buf = vec![0; rand_mtu()];
                buf.fill_with(rand_u8);
                if rand_bool() {
                    let _ = packet_sender.send(Packet::Outgoing(Arc::from(buf.as_slice())));
                } else {
                    let _ = packet_sender.send(Packet::Incoming(Arc::from(buf.as_slice())));
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        })
    }
}

pub struct NoiseChannelManager;
impl ChannelManager for NoiseChannelManager {
    async fn open(&self) -> Arc<dyn Channel> {
        Arc::new(NoiseChannel)
    }
}
