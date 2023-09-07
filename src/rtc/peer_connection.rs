use webrtc::peer_connection::{configuration::RTCConfiguration, RTCPeerConnection};

use super::WEBRTC;

pub async fn create_peer_connection() -> anyhow::Result<RTCPeerConnection> {
    let pc = WEBRTC
        .new_peer_connection(RTCConfiguration::default())
        .await?;
    Ok(pc)
}
