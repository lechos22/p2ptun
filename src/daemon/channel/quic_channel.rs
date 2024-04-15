use std::{io, sync::Arc};

use quinn::{Connection, RecvStream, SendStream};
use tokio::sync::{broadcast, Mutex};

use super::{Channel, ChannelError, Packet};

pub struct QuicChannel {
    send_stream: Arc<Mutex<SendStream>>,
    recv_stream: Arc<Mutex<RecvStream>>,
}

impl QuicChannel {
    pub async fn new(connection: Connection) -> io::Result<Self> {
        let send_stream = connection.open_uni().await?;
        let recv_stream = connection.accept_uni().await?;
        Ok(Self {
            send_stream: Arc::new(Mutex::new(send_stream)),
            recv_stream: Arc::new(Mutex::new(recv_stream)),
        })
    }
}

impl Channel for QuicChannel {
    fn subscribe_packets(
        &self,
        mut packet_receiver: broadcast::Receiver<super::Packet>,
    ) -> super::ChannelJobFuture {
        let send_stream = self.send_stream.clone();
        Box::pin(async move {
            let mut send_stream = send_stream.lock().await;
            loop {
                let packet = packet_receiver.recv().await.unwrap();
                let Packet::Outgoing(packet_content) = packet else {
                    continue;
                };
                match send_stream.write(&packet_content).await {
                    Ok(0) => return Err(ChannelError::Closed),
                    Ok(_) => continue,
                    Err(error) => return Err(ChannelError::Io(error.into())),
                }
            }
        })
    }

    fn publish_packets(
        &self,
        packet_sender: broadcast::Sender<super::Packet>,
    ) -> super::ChannelJobFuture {
        let recv_stream = self.recv_stream.clone();
        Box::pin(async move {
            let mut recv_stream = recv_stream.lock().await;
            loop {
                let mut buffer = vec![0u8; 1518];
                match recv_stream.read(&mut buffer).await {
                    Ok(None) => return Err(ChannelError::Closed),
                    Ok(Some(size)) => {
                        let _ = packet_sender.send(Packet::Incoming(Arc::from(&buffer[..size])));
                    }
                    Err(error) => return Err(ChannelError::Io(error.into())),
                }
            }
        })
    }
}
