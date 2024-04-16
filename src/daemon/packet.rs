use std::{fmt::Debug, sync::Arc};

#[derive(Clone)]
pub enum Packet {
    Outgoing(Arc<[u8]>),
    Incoming(Arc<[u8]>),
}

impl Debug for Packet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Outgoing(arg0) => f.debug_tuple("Outgoing").field(&arg0.len()).finish(),
            Self::Incoming(arg0) => f.debug_tuple("Incoming").field(&arg0.len()).finish(),
        }
    }
}
