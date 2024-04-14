//! A channel for the TUN device.

use std::{
    io::{ErrorKind, Read, Write},
    sync::{Arc, Mutex, TryLockError},
    time::Duration,
};

use tokio::sync::broadcast;
use tun::platform::Device;

use super::{Channel, ChannelError, ChannelJobFuture, ChannelManager, Packet};

pub struct TunChannel {
    device: Arc<Mutex<Device>>,
}

impl Channel for TunChannel {
    fn subscribe_packets(
        &self,
        mut packet_receiver: broadcast::Receiver<Packet>,
    ) -> ChannelJobFuture {
        let device = self.device.clone();
        Box::pin(async move {
            loop {
                let Ok(packet) = packet_receiver.recv().await else {
                    continue;
                };
                if let Packet::Incoming(content) = packet {
                    tun_write_packet(device.clone(), content).await?;
                }
            }
        })
    }

    fn publish_packets(&self, packet_sender: broadcast::Sender<Packet>) -> ChannelJobFuture {
        let device = self.device.clone();
        Box::pin(async move {
            loop {
                let packet_content = tun_read_packet(device.clone()).await?;
                let _ = packet_sender.send(Packet::Outgoing(packet_content));
            }
        })
    }
}

pub struct TunChannelManager;
impl ChannelManager for TunChannelManager {
    async fn open(&self) -> Arc<dyn Channel> {
        let device = tun::create(tun::configure().up()).unwrap();
        device.set_nonblock().unwrap();
        Arc::new(TunChannel {
            device: Arc::new(Mutex::new(device)),
        })
    }
}

async fn tun_write_packet(
    device: Arc<Mutex<Device>>,
    packet_content: Arc<[u8]>,
) -> Result<(), ChannelError> {
    loop {
        tokio::time::sleep(Duration::from_millis(10)).await;
        let mut lock = match device.try_lock() {
            Ok(lock) => lock,
            Err(TryLockError::WouldBlock) => continue,
            Err(TryLockError::Poisoned(_)) => return Err(ChannelError::Poisoned),
        };
        match lock.write(&packet_content) {
            Ok(0) => return Err(ChannelError::Closed),
            Ok(_) => return Ok(()),
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                continue;
            }
            Err(error) => return Err(ChannelError::Io(error)),
        }
    }
}

async fn tun_read_packet(device: Arc<Mutex<Device>>) -> Result<Arc<[u8]>, ChannelError> {
    let mut buffer = [0u8; 1518];
    loop {
        tokio::time::sleep(Duration::from_millis(10)).await;
        let mut lock = match device.try_lock() {
            Ok(lock) => lock,
            Err(TryLockError::WouldBlock) => continue,
            Err(TryLockError::Poisoned(_)) => return Err(ChannelError::Poisoned),
        };
        match lock.read(&mut buffer) {
            Ok(0) => return Err(ChannelError::Closed),
            Ok(size) => return Ok(Arc::from(&buffer[0..size])),
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                continue;
            }
            Err(error) => return Err(ChannelError::Io(error)),
        }
    }
}
