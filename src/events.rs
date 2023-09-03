use std::{error::Error, pin::Pin, sync::mpsc::TryRecvError, task::Poll, time::Duration};

use actix_web::{get, HttpResponse};
use bytes::Bytes;
use futures::Stream;
use lazy_static::lazy_static;
use pub_sub::{PubSub, Subscription};

lazy_static! {
    pub static ref SSE_CHANNEL: PubSub<Sse> = PubSub::new();
}

#[derive(Debug, Clone)]
pub struct Sse {
    event: String,
    data: Option<String>,
}

impl Sse {
    pub fn new(event: String, data: Option<String>) -> Self {
        Self { event, data }
    }
}

trait SseEmitter {
    fn try_recv(&self) -> Result<Sse, TryRecvError>;
}

impl SseEmitter for Subscription<Sse> {
    fn try_recv(&self) -> Result<Sse, TryRecvError> {
        (self as &Subscription<Sse>).try_recv()
    }
}

impl Stream for dyn SseEmitter {
    type Item = Result<Bytes, Box<dyn Error>>;
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match self.try_recv() {
            Err(_) => {
                let waker = cx.waker().clone();
                tokio::spawn(tokio::time::timeout(
                    Duration::from_millis(300),
                    async move {
                        waker.wake();
                    },
                ));
                Poll::Pending
            }
            Ok(sse) => {
                let msg = match sse.data {
                    Some(data) => format!("event: {}\ndata: {}\n\n", sse.event, data),
                    None => format!("event: {}\ndata: \n\n", sse.event)
                };
                let bytes = Bytes::copy_from_slice(msg.as_bytes());
                Poll::Ready(Some(Ok(bytes)))
            }
        }
    }
}

#[get("/subscribe")]
pub async fn subscribe() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/event-stream")
        .streaming(Box::pin(SSE_CHANNEL.subscribe()) as Pin<Box<dyn SseEmitter>>)
}
