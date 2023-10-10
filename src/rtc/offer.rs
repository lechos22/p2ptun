use std::{
    sync::{Arc, Mutex},
    task::Poll,
};

use anyhow::anyhow;
use futures::Future;
use webrtc::{
    data_channel::{data_channel_init::RTCDataChannelInit, RTCDataChannel},
    ice_transport::{
        ice_candidate::{RTCIceCandidate, RTCIceCandidateInit},
        ice_gathering_state::RTCIceGatheringState,
    },
    peer_connection::{sdp::session_description::RTCSessionDescription, RTCPeerConnection},
};

use super::{peer_connection::create_peer_connection, Connection};

struct CreateOffer {
    pc: Arc<RTCPeerConnection>,
    data_channel: Arc<RTCDataChannel>,
    desc: RTCSessionDescription,
    ice_candidates: Arc<Mutex<Vec<RTCIceCandidate>>>,
}

impl CreateOffer {
    async fn new() -> anyhow::Result<Self> {
        let pc = Arc::new(create_peer_connection().await?);
        let data_channel = pc
            .create_data_channel(
                "main",
                Some(RTCDataChannelInit {
                    negotiated: Some(0),
                    ..Default::default()
                }),
            )
            .await?;
        let desc = pc.create_offer(None).await?;
        Ok(Self {
            pc,
            data_channel,
            desc,
            ice_candidates: Arc::new(Mutex::new(Vec::new())),
        })
    }
}

impl Future for CreateOffer {
    type Output = anyhow::Result<Connection>;
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let waker = cx.waker().clone();
        let ice_candidates = self.ice_candidates.clone();
        self.pc.on_ice_candidate(Box::new(move |candidate| {
            let waker = waker.clone();
            let ice_candidates = ice_candidates.clone();
            Box::pin(async move {
                let candidate = candidate.clone();
                if let Some(candidate) = candidate {
                    if let Ok(mut lock) = ice_candidates.lock() {
                        lock.push(candidate)
                    }
                } else {
                    waker.wake()
                }
            })
        }));
        let pc_2 = self.pc.clone();
        let offer_2 = self.desc.clone();
        tokio::spawn(async move {
            if pc_2.local_description().await.is_none() {
                let _ = pc_2.set_local_description(offer_2).await;
            }
        });
        match self.pc.ice_gathering_state() {
            RTCIceGatheringState::Complete => match self.ice_candidates.lock() {
                Ok(lock) => Poll::Ready(Ok(Connection {
                    pc: self.pc.clone(),
                    data_channel: self.data_channel.clone(),
                    desc: self.desc.clone(),
                    ice_candidates: lock.clone(),
                })),
                Err(_) => Poll::Ready(Err(anyhow!("Mutex was poisoned"))),
            },
            _ => Poll::Pending,
        }
    }
}

pub async fn create_offer() -> anyhow::Result<Connection> {
    CreateOffer::new().await?.await
}

pub async fn accept_answer(
    connection_init: Connection,
    answer: RTCSessionDescription,
    ice_candidates: Vec<RTCIceCandidateInit>,
) -> anyhow::Result<()> {
    connection_init.pc.set_remote_description(answer).await?;
    for candidate in ice_candidates {
        connection_init.pc.add_ice_candidate(candidate).await?;
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use webrtc::peer_connection::sdp::sdp_type::RTCSdpType;

    use super::create_offer;

    #[tokio::test]
    async fn test_creation() {
        create_offer().await.unwrap();
    }

    #[tokio::test]
    async fn test_offer_sdp() {
        let desc = create_offer().await.unwrap().desc;
        assert_eq!(desc.sdp_type, RTCSdpType::Offer);
        assert_eq!(desc.sdp.is_empty(), false);
    }

    #[tokio::test]
    async fn test_data_channel() {
        let data_channel = create_offer().await.unwrap().data_channel;
        assert_eq!(data_channel.negotiated(), true);
    }

    #[tokio::test]
    async fn test_ice_candidates() {
        let ice_candidates = create_offer().await.unwrap().ice_candidates;
        assert_eq!(ice_candidates.is_empty(), false);
    }
}
