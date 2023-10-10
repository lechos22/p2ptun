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

struct CreateAnswer {
    pc: Arc<RTCPeerConnection>,
    data_channel: Arc<RTCDataChannel>,
    desc: RTCSessionDescription,
    ice_candidates: Arc<Mutex<Vec<RTCIceCandidate>>>,
}

impl CreateAnswer {
    async fn new(
        offer: RTCSessionDescription,
        ice_candidates: Vec<RTCIceCandidateInit>,
    ) -> anyhow::Result<Self> {
        let pc = Arc::new(create_peer_connection().await?);
        pc.set_remote_description(offer).await?;
        for candidate in ice_candidates {
            pc.add_ice_candidate(candidate).await?;
        }
        let data_channel = pc
            .create_data_channel(
                "main",
                Some(RTCDataChannelInit {
                    negotiated: Some(0),
                    ..Default::default()
                }),
            )
            .await?;
        let desc = pc.create_answer(None).await?;
        Ok(Self {
            pc,
            data_channel,
            desc,
            ice_candidates: Arc::new(Mutex::new(Vec::new())),
        })
    }
}

impl Future for CreateAnswer {
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
        let answer_2 = self.desc.clone();
        tokio::spawn(async move {
            if pc_2.local_description().await.is_none() {
                let _ = pc_2.set_local_description(answer_2).await;
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

pub async fn create_answer(
    offer: RTCSessionDescription,
    ice_candidates: Vec<RTCIceCandidateInit>,
) -> anyhow::Result<Connection> {
    CreateAnswer::new(offer, ice_candidates).await?.await
}

#[cfg(test)]
mod test {
    use crate::rtc::offer::create_offer;

    use super::create_answer;

    #[tokio::test]
    async fn test_creation() {
        let offer = create_offer().await.unwrap();
        let offer_candidates = offer
            .ice_candidates
            .iter()
            .map(|candidate| candidate.to_json().unwrap())
            .collect::<Vec<_>>();
        create_answer(offer.desc, offer_candidates).await.unwrap();
    }
}
