use webrtc::peer_connection::{configuration::RTCConfiguration, RTCPeerConnection};

use super::WEBRTC;

pub async fn create_peer_connection() -> anyhow::Result<RTCPeerConnection> {
    let pc = WEBRTC
        .new_peer_connection(RTCConfiguration::default())
        .await?;
    Ok(pc)
}

#[cfg(test)]
mod test {
    use crate::rtc::peer_connection::create_peer_connection;

    #[tokio::test]
    async fn test_creation() {
        create_peer_connection().await.unwrap();
    }
}
