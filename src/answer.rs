use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use uuid::Uuid;
use webrtc::{
    data_channel::data_channel_init::RTCDataChannelInit,
    ice_transport::{
        ice_candidate::RTCIceCandidateInit, ice_gathering_state::RTCIceGatheringState,
    },
    peer_connection::{
        configuration::RTCConfiguration, offer_answer_options::RTCAnswerOptions,
        sdp::session_description::RTCSessionDescription,
    },
};

use crate::{
    apply_data_channel_handlers, connection::IcedSessionDescription, create_ice_candidate_handler,
    errors::WrapErrors, WEBRTC,
};

pub async fn create_answer_inner(
    offer: IcedSessionDescription,
) -> Result<IcedSessionDescription, String> {
    let pc = WEBRTC
        .new_peer_connection(RTCConfiguration::default())
        .await
        .wrap_errors()?;
    let candidates = Arc::new(Mutex::new(Vec::<RTCIceCandidateInit>::new()));
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
    pc.on_ice_candidate(create_ice_candidate_handler(candidates.clone()));
    let IcedSessionDescription::Offer {
        sdp: offer_sdp,
        ice_candidates: remote_candidates,
    } = offer
    else {
        return Err("Offer isn't an offer".to_string());
    };
    pc.set_remote_description(RTCSessionDescription::offer(offer_sdp).wrap_errors()?)
        .await
        .wrap_errors()?;
    for candidate in remote_candidates {
        pc.add_ice_candidate(candidate).await.wrap_errors()?;
    }
    let answer = pc
        .create_answer(Some(RTCAnswerOptions::default()))
        .await
        .wrap_errors()?;
    let answer_sdp = answer.sdp.clone();
    pc.set_local_description(answer).await.wrap_errors()?;
    while pc.ice_gathering_state() == RTCIceGatheringState::Gathering {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let candidates = candidates.lock().wrap_errors()?.clone();
    let iced_offer = IcedSessionDescription::Answer {
        sdp: answer_sdp,
        ice_candidates: candidates,
    };
    Ok(iced_offer)
}
