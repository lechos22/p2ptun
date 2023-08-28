#![allow(dead_code)]

mod answer;
mod args;
mod connection;
mod errors;
mod offer;

use std::{future::Future, pin::Pin, sync::Arc, net::Ipv6Addr};

use actix_web::{get, post, App, HttpResponse, HttpServer};
use args::Args;
use clap::Parser;
use connection::{Connection, CONNECTIONS};
use lazy_static::lazy_static;
use tokio::sync::Mutex;
use uuid::Uuid;
use webrtc::{
    data_channel::RTCDataChannel,
    ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit},
};

use errors::WrapErrors;

use crate::{
    answer::create_answer_inner,
    connection::{IcedSessionDescription, IPV6_ADDRESS},
    offer::{accept_answer_inner, create_offer_inner},
};

lazy_static! {
    pub static ref WEBRTC: webrtc::api::API = webrtc::api::APIBuilder::new().build();
}

async fn get_peers_inner() -> Result<Vec<(String, Option<Ipv6Addr>)>, String> {
    let connections = CONNECTIONS.lock().await;
    let mut peers = Vec::with_capacity(connections.len());
    for (id, con) in connections.iter() {
        peers.push((id.to_string(), con.lock().await.get_ip()));
    }
    Ok(peers)
}

#[get("/peers")]
async fn get_peers() -> HttpResponse {
    match get_peers_inner().await {
        Ok(peers) => match serde_json::to_string(&peers) {
            Ok(json) => HttpResponse::Ok()
                .content_type("application/json")
                .body(json),
            Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
        },
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

fn apply_data_channel_handlers(id: Uuid, data_channel: Arc<RTCDataChannel>) {
    let id_c = id.clone();
    let data_channel_c = data_channel.clone();
    data_channel.on_open(Box::new(move || {
        Box::pin(async move {
            println!("Opened connection {}", id);
            CONNECTIONS
                .lock()
                .await
                .insert(id, Mutex::new(Connection::new(id_c, data_channel_c)));
        })
    }));
    let id_c = id.clone();
    data_channel.on_message(Box::new(move |message| {
        Box::pin(async move {
            if let Some(conn) = CONNECTIONS.lock().await.get(&id_c) {
                conn.lock().await.handle_message(message);
            }
        })
    }));
    let id_c = id.clone();
    data_channel.on_close(Box::new(move || {
        Box::pin(async move {
            println!("Closed connection {}", id);
            CONNECTIONS.lock().await.remove(&id_c);
        })
    }));
}

type PinnedFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
type IceCandidateHandler = Box<dyn FnMut(Option<RTCIceCandidate>) -> PinnedFuture + Send + Sync>;

fn create_ice_candidate_handler(
    candidates: Arc<std::sync::Mutex<Vec<RTCIceCandidateInit>>>,
) -> IceCandidateHandler {
    Box::new(move |candidate| {
        let candidates = candidates.clone();
        Box::pin(async move {
            if let Some(Ok(candidate)) = candidate.map(|i| i.to_json()) {
                if let Ok(mut lock) = candidates.lock() {
                    lock.push(candidate);
                }
            }
        })
    })
}

#[get("/call/offer")]
async fn create_offer() -> HttpResponse {
    match create_offer_inner().await {
        Ok(body) => match serde_json::to_string(&body).wrap_errors() {
            Ok(body) => HttpResponse::Ok()
                .content_type("application/json")
                .body(body),
            Err(body) => HttpResponse::InternalServerError().body(body),
        },
        Err(body) => HttpResponse::InternalServerError().body(body),
    }
}

#[post("/call/answer")]
async fn create_answer(offer: String) -> HttpResponse {
    match serde_json::from_str::<IcedSessionDescription>(&offer) {
        Ok(offer) => match create_answer_inner(offer).await {
            Ok(body) => match serde_json::to_string(&body).wrap_errors() {
                Ok(body) => HttpResponse::Ok()
                    .content_type("application/json")
                    .body(body),
                Err(body) => HttpResponse::InternalServerError().body(body),
            },
            Err(body) => HttpResponse::InternalServerError().body(body),
        },
        Err(err) => HttpResponse::BadRequest().body(err.to_string()),
    }
}

#[post("/call/accept")]
async fn accept_answer(answer: String) -> HttpResponse {
    match serde_json::from_str::<IcedSessionDescription>(&answer) {
        Ok(answer) => match accept_answer_inner(answer).await {
            Ok(()) => HttpResponse::Ok().finish(),
            Err(body) => HttpResponse::InternalServerError().body(body),
        },
        Err(err) => HttpResponse::BadRequest().body(err.to_string()),
    }
}

#[get("/ip/v6")]
async fn get_ip() -> HttpResponse {
    HttpResponse::Ok().body(IPV6_ADDRESS.to_string())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    connection::listen_tun();
    HttpServer::new(|| {
        App::new()
            .service(get_peers)
            .service(create_offer)
            .service(create_answer)
            .service(accept_answer)
            .service(get_ip)
    })
    .bind(("127.0.0.1", args.get_port()))?
    .run()
    .await
}
