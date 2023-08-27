use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use lazy_static::lazy_static;
use uuid::Uuid;
use webrtc::{
    data_channel::data_channel_init::RTCDataChannelInit,
    ice_transport::{
        ice_candidate::RTCIceCandidateInit, ice_gathering_state::RTCIceGatheringState,
    },
    peer_connection::{
        configuration::RTCConfiguration, offer_answer_options::RTCOfferOptions,
        sdp::session_description::RTCSessionDescription, RTCPeerConnection,
    },
};

use crate::{
    apply_data_channel_handlers, connection::IcedSessionDescription, create_ice_candidate_handler,
    errors::WrapErrors, WEBRTC,
};

lazy_static! {
    static ref WAITING_CONNECTION: Mutex<Option<Arc<Mutex<RTCPeerConnection>>>> = Mutex::new(None);
}

pub async fn create_offer_inner() -> Result<IcedSessionDescription, String> {
    let pc = WEBRTC
        .new_peer_connection(RTCConfiguration::default())
        .await
        .wrap_errors()?;
    let candidates = Arc::new(Mutex::new(Vec::<RTCIceCandidateInit>::new()));
    pc.on_ice_candidate(create_ice_candidate_handler(candidates.clone()));
    let data_channel = pc
        .create_data_channel(
            "socket_data",
            Some(RTCDataChannelInit {
                negotiated: Some(1),
                ..Default::default()
            }),
        )
        .await
        .wrap_errors()?;
    apply_data_channel_handlers(Uuid::new_v4(), data_channel.clone());
    let offer = pc
        .create_offer(Some(RTCOfferOptions::default()))
        .await
        .wrap_errors()?;
    let offer_sdp = offer.sdp.clone();
    pc.set_local_description(offer).await.wrap_errors()?;
    while pc.ice_gathering_state() == RTCIceGatheringState::Gathering {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let candidates = candidates.lock().wrap_errors()?.clone();
    let iced_offer = IcedSessionDescription::Offer {
        sdp: offer_sdp,
        ice_candidates: candidates,
    };
    if let Ok(mut lock) = WAITING_CONNECTION.lock() {
        *lock = Some(Arc::new(Mutex::new(pc)));
    }
    Ok(iced_offer)
}

pub async fn accept_answer_inner(answer: IcedSessionDescription) -> Result<(), String> {
    let IcedSessionDescription::Answer {
        sdp: answer_sdp,
        ice_candidates: remote_candidates,
    } = answer
    else {
        return Err("Answer isn't an answer".to_string());
    };
    if let Ok(mut lock) = WAITING_CONNECTION.lock() {
        if let Some(value) = lock.clone() {
            if let Ok(lock) = value.lock() {
                lock.set_remote_description(
                    RTCSessionDescription::answer(answer_sdp).wrap_errors()?,
                )
                .await
                .wrap_errors()?;
                for candidate in remote_candidates {
                    lock.add_ice_candidate(candidate).await.wrap_errors()?;
                }
            }
        }
        *lock = None;
    }
    Ok(())
}
