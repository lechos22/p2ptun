use std::collections::HashMap;

use iroh_net::{
    dialer::Dialer, key::SecretKey, relay::RelayMode, tls::certificate::P2pCertificate,
    MagicEndpoint, NodeAddr, NodeId,
};
use quinn::Connecting;
use tokio::{
    sync::{broadcast, mpsc},
    task::JoinSet,
};

use crate::daemon::channel::{quic_channel::QuicChannel, Channel, ChannelError, Packet};

use super::ChannelGroup;

const ALPN: &[u8] = "p2ptun".as_bytes();

pub enum IrohChannelGroupCommand {
    AddPeer(NodeAddr),
    AddQuicChannel(NodeId, QuicChannel),
}

pub struct IrohChannelGroup {
    command_receiver: mpsc::Receiver<IrohChannelGroupCommand>,
    command_sender: mpsc::Sender<IrohChannelGroupCommand>,
    magic_endpoint: MagicEndpoint,
    peer_map: HashMap<NodeId, JoinSet<Result<(), ChannelError>>>,
}

impl IrohChannelGroup {
    pub async fn new(secret_key: SecretKey) -> Self {
        let magic_endpoint = MagicEndpoint::builder()
            .secret_key(secret_key)
            .alpns(vec![ALPN.to_vec()])
            .relay_mode(RelayMode::Default)
            .bind(0)
            .await
            .unwrap();
        let (command_sender, command_receiver) = mpsc::channel(8);
        Self {
            command_receiver,
            command_sender,
            magic_endpoint,
            peer_map: HashMap::new(),
        }
    }
    pub fn get_command_sender(&self) -> mpsc::Sender<IrohChannelGroupCommand> {
        self.command_sender.clone()
    }
    async fn task_dial_peer(
        magic_endpoint: MagicEndpoint,
        command_sender: mpsc::Sender<IrohChannelGroupCommand>,
        node_id: NodeId,
    ) {
        let mut dialer = Dialer::new(magic_endpoint);
        dialer.queue_dial(node_id, ALPN);
        let (_, Ok(connection)) = dialer.next_conn().await else {
            return;
        };
        let Ok(channel) = QuicChannel::new(connection).await else {
            return;
        };
        let _ = command_sender
            .send(IrohChannelGroupCommand::AddQuicChannel(node_id, channel))
            .await;
    }
    async fn task_add_peer(
        connecting: Connecting,
        command_sender: mpsc::Sender<IrohChannelGroupCommand>,
    ) {
        let Ok(connection) = connecting.await else {
            return;
        };
        let Some(peer_identity) = connection.peer_identity() else {
            return;
        };
        let Ok(certificate) = peer_identity.downcast::<P2pCertificate>() else {
            return;
        };
        let Ok(channel) = QuicChannel::new(connection).await else {
            return;
        };
        let _ = command_sender.send(IrohChannelGroupCommand::AddQuicChannel(
            certificate.peer_id(),
            channel,
        ));
    }
    async fn task_accept_peers(
        magic_endpoint: MagicEndpoint,
        command_sender: mpsc::Sender<IrohChannelGroupCommand>,
    ) {
        while let Some(connecting) = magic_endpoint.accept().await {
            tokio::spawn(IrohChannelGroup::task_add_peer(
                connecting,
                command_sender.clone(),
            ));
        }
    }
    async fn run_command(
        &mut self,
        packet_sender: &broadcast::Sender<Packet>,
        command: IrohChannelGroupCommand,
    ) {
        match command {
            IrohChannelGroupCommand::AddPeer(node_addr) => {
                if let Err(error) = self.magic_endpoint.add_node_addr(node_addr.clone()) {
                    eprintln!("{:?}", error);
                    return;
                };
                tokio::spawn(IrohChannelGroup::task_dial_peer(
                    self.magic_endpoint.clone(),
                    self.get_command_sender(),
                    node_addr.node_id,
                ));
            }
            IrohChannelGroupCommand::AddQuicChannel(node_id, channel) => {
                eprintln!("Connected to {}", node_id);
                let mut join_set = JoinSet::new();
                join_set.spawn(channel.subscribe_packets(packet_sender.subscribe()));
                join_set.spawn(channel.publish_packets(packet_sender.clone()));
                self.peer_map.insert(node_id, join_set);
            }
        }
    }
}

impl ChannelGroup for IrohChannelGroup {
    async fn run(mut self, packet_sender: broadcast::Sender<Packet>) -> ! {
        tokio::spawn(IrohChannelGroup::task_accept_peers(
            self.magic_endpoint.clone(),
            self.get_command_sender(),
        ));
        loop {
            let command = self.command_receiver.recv().await.unwrap();
            self.run_command(&packet_sender, command).await;
        }
    }
}
