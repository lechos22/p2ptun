use std::{collections::HashMap, net::Ipv6Addr, sync::Arc, time::Duration};

use bytes::Bytes;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tun_tap::Iface;
use uuid::Uuid;
use webrtc::{
    data_channel::{data_channel_message::DataChannelMessage, RTCDataChannel},
    ice_transport::ice_candidate::RTCIceCandidateInit,
};

use crate::errors::WrapErrors;

const fn nth_group(x: u128, n: u8) -> u16 {
    ((x & (0xffff << (n * 16))) >> (n * 16)) as u16
}

lazy_static! {
    pub static ref CONNECTIONS: Mutex<HashMap<Uuid, Mutex<Connection>>> =
        Mutex::new(HashMap::new());
    pub static ref IPV6_ADDRESS: Ipv6Addr = {
        let id = Uuid::new_v4().as_u128();
        Ipv6Addr::new(
            0xfc00,
            nth_group(id, 6),
            nth_group(id, 5),
            nth_group(id, 4),
            nth_group(id, 3),
            nth_group(id, 2),
            nth_group(id, 1),
            nth_group(id, 0),
        )
    };
    pub static ref IFACE: Iface = {
        let iface = Iface::new("p2ptun", tun_tap::Mode::Tun).unwrap();
        iface.set_non_blocking().unwrap();
        //iface.send(&[]).unwrap();
        iface
    };
}

#[derive(Serialize, Deserialize, Clone)]
pub enum IcedSessionDescription {
    Offer {
        sdp: String,
        ice_candidates: Vec<RTCIceCandidateInit>,
    },
    Answer {
        sdp: String,
        ice_candidates: Vec<RTCIceCandidateInit>,
    },
}

pub struct Connection {
    id: Uuid,
    ip: Option<Ipv6Addr>,
    data_channel: Arc<RTCDataChannel>,
}

impl Connection {
    pub fn new(id: Uuid, data_channel: Arc<RTCDataChannel>) -> Connection {
        let data_channel_c = data_channel.clone();
        tokio::spawn(async move {
            let _ = data_channel_c
                .send_text(format!("IP {}", IPV6_ADDRESS.to_string()))
                .await;
        });
        Connection {
            id,
            ip: None,
            data_channel,
        }
    }
    pub fn get_ip(&self) -> Option<Ipv6Addr> {
        self.ip
    }
    pub async fn send_text(&self, message: String) -> Result<(), String> {
        self.data_channel.send_text(message).await.wrap_errors()?;
        Ok(())
    }
    pub async fn send(&self, message: &Bytes) -> Result<(), String> {
        let size = self.data_channel.send(message).await.wrap_errors()?;
        println!("Sending {} bytes to {}, IP: {:?}", size, self.id, self.ip);
        Ok(())
    }
    pub fn handle_message(&mut self, message: DataChannelMessage) {
        if message.is_string {
            match String::from_utf8(message.data.to_vec()) {
                Ok(message) if message.starts_with("IP fc") => match message[3..].parse() {
                    Ok(ip) => {
                        println!("Made connection with {}", ip);
                        self.ip = Some(ip)
                    }
                    _ => {}
                },
                _ => {}
            }
        } else {
            match IFACE.send(&message.data) {
                Ok(size) => println!("Receiving {} bytes from {}, IP: {:?}", size, self.id, self.ip),
                Err(err) => eprintln!("TUN error {}", err.to_string()),
            }
        }
    }
}

pub async fn publish(msg: &Bytes) -> Result<(), String> {
    for (_, conn) in CONNECTIONS.lock().await.iter() {
        conn.lock().await.send(msg).await?;
    }
    Ok(())
}

pub fn listen_tun() -> () {
    tokio::spawn(async {
        let mut buf: [u8; 1542] = [0; 1542];
        loop {
            match IFACE.recv(&mut buf) {
                Ok(size) => {
                    let buf = buf[..size].to_vec();
                    if let Err(err) = publish(&Bytes::copy_from_slice(&buf)).await {
                        eprintln!("TUN error {}", err.to_string());
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(err) => {
                    eprintln!("TUN error {}", err.to_string());
                }
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    });
}
