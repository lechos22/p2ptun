use std::{fmt::Debug, sync::Arc};

use once_cell::sync::Lazy;
use webrtc::{
    data_channel::RTCDataChannel,
    ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit},
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
    pub fn get_desc(&self) -> RTCSessionDescription {
        self.desc.clone()
    }
    pub fn get_data_channel(&self) -> Arc<RTCDataChannel> {
        self.data_channel.clone()
    }
    pub fn get_ice_candidates(&self) -> Vec<RTCIceCandidateInit> {
        self.ice_candidates
            .iter()
            .filter_map(|candidate| candidate.to_json().ok())
            .collect::<Vec<_>>()
    }
}

impl Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {{ desc: {:?}, ice_candidates: {:?} }}",
            stringify!(Connection),
            self.desc,
            self.ice_candidates
        )
    }
}
