//! This module defines abstractions for actors and message passing.
//!
//! Actors can receive messages through addresses ([Addr]) and handle them asynchronously.
//! Each actor implements the [Actor] trait, allowing it to send and receive messages.

pub mod packet_logger;
pub mod packet_router;
pub mod tun;

use std::fmt::Debug;

use tokio::sync::mpsc;

/// Represents the address of an actor to which messages can be sent.
#[derive(Debug, Clone)]
pub struct Addr<Message> {
    /// The message sender of this actor
    sender: mpsc::Sender<Message>,
}

impl<Message> Addr<Message> {
    /// Creates an address from a [`mpsc::Sender`]
    pub fn new(sender: mpsc::Sender<Message>) -> Self {
        Self { sender }
    }
    /// Sends a message to the addressed actor
    pub async fn send_message(&self, message: Message) {
        let _ = self.sender.send(message).await;
    }
}

/// A trait implemented by actors that can receive messages.
///
/// Actors implementing this trait provide an [Addr] that can be used to send messages to them.
pub trait Actor<Message>: Send + Sync
where
    Message: Send,
{
    /// Returns the actor's address
    fn get_addr(&self) -> Addr<Message>;
}
