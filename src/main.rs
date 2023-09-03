#![allow(dead_code)]

mod answer;
mod args;
mod connection;
mod errors;
mod offer;
mod tun;
mod webui;
mod events;

use std::{fmt::Display, future::Future, pin::Pin, sync::Arc};

use actix_web::{
    dev::Service,
    get,
    middleware::{Logger, NormalizePath},
    post, App, HttpResponse, HttpServer,
};
use args::Args;
use clap::Parser;
use connection::{publish, Connection, CONNECTIONS, TUN};
use lazy_static::lazy_static;
use log::info;
use tokio::sync::Mutex;
use uuid::Uuid;
use webrtc::{
    data_channel::RTCDataChannel,
    ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit},
};

use errors::WrapErrors;
use webui::WebUI;

use crate::{
    answer::create_answer_inner,
    connection::IcedSessionDescription,
    offer::{accept_answer_inner, create_offer_inner}, events::{subscribe, SSE_CHANNEL, Sse},
};

lazy_static! {
    pub static ref WEBRTC: webrtc::api::API = webrtc::api::APIBuilder::new().build();
}

async fn get_peers_inner() -> Result<Vec<String>, String> {
    let connections = CONNECTIONS.lock().await;
    let mut peers = Vec::with_capacity(connections.len());
    for (id, _con) in connections.iter() {
        peers.push(id.to_string());
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
    let id = id;
    let data_channel_weak = Arc::downgrade(&data_channel);
    data_channel.on_open(Box::new(move || {
        Box::pin(async move {
            if let Some(data_channel) = data_channel_weak.upgrade() {
                info!("Opened connection {}", id);
                let _ = SSE_CHANNEL.send(Sse::new("peers:new".to_string(), None));
                CONNECTIONS
                    .lock()
                    .await
                    .insert(id, Mutex::new(Connection::new(id, data_channel)));
            }
        })
    }));
    data_channel.on_message(Box::new(move |message| {
        Box::pin(async move {
            if let Some(conn) = CONNECTIONS.lock().await.get(&id) {
                conn.lock().await.handle_message(message);
            }
        })
    }));
    data_channel.on_error(Box::new(move |err| {
        Box::pin(async move {
            info!("Closed connection {} due to error {}", id, err);
            CONNECTIONS.lock().await.remove(&id);
        })
    }));
    data_channel.on_close(Box::new(move || {
        Box::pin(async move {
            info!("Closed connection {}", id);
            CONNECTIONS.lock().await.remove(&id);
        })
    }));
}

type PinnedFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
type IceCandidateHandler = Box<dyn FnMut(Option<RTCIceCandidate>) -> PinnedFuture + Send + Sync>;

fn create_ice_candidate_handler(
    candidates: Arc<Mutex<Vec<RTCIceCandidateInit>>>,
) -> IceCandidateHandler {
    Box::new(move |candidate| {
        let candidates = candidates.clone();
        Box::pin(async move {
            if let Some(Ok(candidate)) = candidate.map(|i| i.to_json()) {
                let mut lock = candidates.lock().await;
                lock.push(candidate);
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

#[derive(Debug, Default)]
struct BadRequestSourceError {}
impl actix_web::ResponseError for BadRequestSourceError {}
impl Display for BadRequestSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bad request source")
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    TUN.listen(Box::new(|buf| {
        Box::pin(async move {
            if let Err(err) = publish(buf).await {
                eprintln!("TUN error {}", err);
            }
        })
    }))
    .await;
    let port = args.get_port();
    info!("RPC API listening on http://localhost:{}", port);
    info!("Web UI available on http://localhost:{}/webui", port);
    HttpServer::new(|| {
        App::new()
            .wrap(NormalizePath::new(
                actix_web::middleware::TrailingSlash::Trim,
            ))
            .wrap(Logger::new(
                r#"%a "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#,
            ))
            .wrap_fn(|req, srv| {
                let addr = req.peer_addr();
                let fut = srv.call(req);
                async move {
                    match addr {
                        Some(addr) if addr.ip().is_loopback() => Ok(fut.await?),
                        _ => Err(actix_web::Error::from(BadRequestSourceError::default())),
                    }
                }
            })
            .service(get_peers)
            .service(create_offer)
            .service(create_answer)
            .service(accept_answer)
            .service(subscribe)
            .webui()
    })
    .bind(("localhost", port))?
    .run()
    .await?;
    Ok(())
}
