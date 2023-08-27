use std::sync::Arc;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use webrtc::{
    data_channel::{data_channel_message::DataChannelMessage, RTCDataChannel},
    ice_transport::ice_candidate::RTCIceCandidateInit,
};

use crate::errors::WrapErrors;

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
        Connection { id, data_channel }
    }
    pub async fn send_text(&self, message: String) -> Result<(), String> {
        self.data_channel.send_text(message).await.wrap_errors()?;
        Ok(())
    }
    pub async fn send(&self, message: &Bytes) -> Result<(), String> {
        self.data_channel.send(message).await.wrap_errors()?;
        Ok(())
    }
    pub fn handle_message(&self, message: DataChannelMessage) {
        println!("{:?}", message);
    }
}
