use std::{fmt::Debug, sync::Arc};

use once_cell::sync::Lazy;
use webrtc::{
    data_channel::RTCDataChannel,
    ice_transport::ice_candidate::RTCIceCandidate,
    peer_connection::{sdp::session_description::RTCSessionDescription, RTCPeerConnection},
};

pub mod answer;
pub mod offer;
pub mod peer_connection;

static WEBRTC: Lazy<webrtc::api::API> = Lazy::new(|| webrtc::api::APIBuilder::new().build());

pub struct Connection {
    pc: Arc<RTCPeerConnection>,
    desc: RTCSessionDescription,
    data_channel: Arc<RTCDataChannel>,
    ice_candidates: Vec<RTCIceCandidate>,
}

impl Connection {
    pub fn get_data_channel(&self) -> Arc<RTCDataChannel> {
        self.data_channel.clone()
    }
}

impl Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ConnectionInit {{ pc: {:?}, desc: {:?}, data_channel: [...], ice_candidates: {:?} }}",
            self.pc, self.desc, self.ice_candidates
        )
    }
}
