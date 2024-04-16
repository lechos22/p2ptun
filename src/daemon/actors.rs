use std::fmt::Debug;

use tokio::sync::mpsc;

pub mod packet_logger;
pub mod packet_router;
pub mod tun;

#[derive(Debug, Clone)]
pub struct Addr<Message> {
    sender: mpsc::Sender<Message>,
}

impl<Message> Addr<Message> {
    pub fn new(sender: mpsc::Sender<Message>) -> Self {
        Self { sender }
    }
    pub async fn send_message(&self, message: Message) {
        let _ = self.sender.send(message).await;
    }
}

pub trait Actor<Message>: Send + Sync
where
    Message: Send,
{
    fn get_addr(&self) -> Addr<Message>;
}
