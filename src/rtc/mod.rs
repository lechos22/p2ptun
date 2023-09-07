use std::sync::Arc;

use once_cell::sync::Lazy;
use webrtc::{peer_connection::{RTCPeerConnection, sdp::session_description::RTCSessionDescription}, ice_transport::ice_candidate::RTCIceCandidate, data_channel::RTCDataChannel};

pub mod answer;
pub mod offer;
pub mod peer_connection;

static WEBRTC: Lazy<webrtc::api::API> = Lazy::new(|| webrtc::api::APIBuilder::new().build());

pub struct ConnectionInit {
    pc: Arc<RTCPeerConnection>,
    desc: RTCSessionDescription,
    data_channel: Arc<RTCDataChannel>,
    ice_candidates: Vec<RTCIceCandidate>,
}
