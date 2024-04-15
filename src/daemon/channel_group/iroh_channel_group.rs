use std::{collections::HashMap, future::Future};

use iroh_net::{dialer::Dialer, relay::RelayMode, MagicEndpoint, NodeId};
use tokio::{sync::broadcast, task::JoinSet};

use crate::daemon::channel::{quic_channel::QuicChannel, Channel, ChannelError, Packet};

use super::ChannelGroup;

const ALPN: &[u8] = "p2ptun".as_bytes();

pub struct IrohChannelGroup {
    dialer: Dialer,
    magic_endpoint: MagicEndpoint,
    peer_map: HashMap<NodeId, JoinSet<Result<(), ChannelError>>>,
}

impl IrohChannelGroup {
    pub async fn new() -> Self {
        let magic_endpoint = MagicEndpoint::builder()
            .alpns(vec![ALPN.to_vec()])
            .relay_mode(RelayMode::Default)
            .bind(0)
            .await
            .unwrap();
        let dialer = Dialer::new(magic_endpoint.clone());
        Self {
            dialer,
            magic_endpoint,
            peer_map: HashMap::new(),
        }
    }
}

impl ChannelGroup for IrohChannelGroup {
    fn run(mut self, packet_sender: broadcast::Sender<Packet>) -> impl Future<Output = !> + Send {
        async move {
            loop {
                let (node_id, Ok(connection)) = self.dialer.next_conn().await else {
                    continue;
                };
                let Ok(channel) = QuicChannel::new(connection).await else {
                    continue;
                };
                eprintln!("Connected to {}", node_id);
                let mut join_set = JoinSet::new();
                join_set.spawn(channel.subscribe_packets(packet_sender.subscribe()));
                join_set.spawn(channel.publish_packets(packet_sender.clone()));
                self.peer_map.insert(node_id, join_set);
            }
        }
    }
}
