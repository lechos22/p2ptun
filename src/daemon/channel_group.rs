pub mod iroh_channel_group;

use std::future::Future;

use tokio::sync::broadcast;

use super::channel::Packet;

pub trait ChannelGroup {
    fn run(self, packet_sender: broadcast::Sender<Packet>) -> impl Future<Output = !> + Send;
}
