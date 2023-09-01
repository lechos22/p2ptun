use std::{collections::HashMap, sync::Arc};

use bytes::Bytes;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;
use webrtc::{
    data_channel::{data_channel_message::DataChannelMessage, RTCDataChannel},
    ice_transport::ice_candidate::RTCIceCandidateInit,
};

use crate::{errors::WrapErrors, tun::Tun};

const fn nth_group(x: u128, n: u8) -> u16 {
    ((x & (0xffff << (n * 16))) >> (n * 16)) as u16
}

lazy_static! {
    pub static ref CONNECTIONS: Mutex<HashMap<Uuid, Mutex<Connection>>> =
        Mutex::new(HashMap::new());
    pub static ref TUN: Tun = Tun::new("p2ptun%d").unwrap();
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
    data_channel: Arc<RTCDataChannel>,
}

impl Connection {
    pub fn new(id: Uuid, data_channel: Arc<RTCDataChannel>) -> Connection {
        let data_channel_c = data_channel.clone();
        tokio::spawn(async move {
            let _data_channel = data_channel_c;
        });
        Connection { id, data_channel }
    }
    pub async fn send_text(&self, message: String) -> Result<(), String> {
        self.data_channel.send_text(message).await.wrap_errors()?;
        Ok(())
    }
    pub async fn send(&self, message: &Bytes) -> Result<(), String> {
        let size = self.data_channel.send(message).await.wrap_errors()?;
        println!("Sending {} bytes to {}", size, self.id);
        Ok(())
    }
    pub fn handle_message(&mut self, message: DataChannelMessage) {
        if message.is_string {
            let _ = String::from_utf8(message.data.to_vec());
        } else {
            match TUN.send(&message.data) {
                Ok(size) => println!("Receiving {} bytes from {}", size, self.id),
                Err(err) => eprintln!("TUN error {}", err),
            }
        }
    }
}

pub async fn publish(msg: Vec<u8>) -> Result<(), String> {
    for (_, conn) in CONNECTIONS.lock().await.iter() {
        conn.lock().await.send(&Bytes::copy_from_slice(&msg)).await?;
    }
    Ok(())
}
