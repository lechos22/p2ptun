use std::{future::Future, pin::Pin, sync::Arc};

use iroh_net::{MagicEndpoint, PeerAddr};
use quinn::{Connection, SendStream};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    sync::Mutex,
};

use crate::{peer_arddr::parse_peer_addr, constants::P2PTUN_ALPN};

pub struct ApplicationState {
    tun_write: Mutex<WriteHalf<tun::AsyncDevice>>,
    packet_listeners: Mutex<Vec<SendStream>>,
    endpoint: MagicEndpoint,
}

impl ApplicationState {
    pub fn create(endpoint: MagicEndpoint, tun_write: WriteHalf<tun::AsyncDevice>) -> Self {
        Self {
            tun_write: Mutex::new(tun_write),
            packet_listeners: Default::default(),
            endpoint,
        }
    }
}

pub trait Application {
    fn add_connection(&self, connection: Connection);
    fn open_stream(&self, connection: Connection);
    fn accept_stream(&self, connection: Connection);
    fn accept_incoming_connections(&self);
    fn send_outcoming_packets(&self, tun_read: ReadHalf<tun::AsyncDevice>);
    fn get_addr(&self) -> Pin<Box<dyn Future<Output = anyhow::Result<PeerAddr>>>>;
    fn add_peer_from_address(&self, address: String);
}

impl Application for Arc<ApplicationState> {
    fn add_connection(&self, con: Connection) {
        eprintln!("Connecting to {}", con.remote_address());
        self.open_stream(con.clone());
        self.accept_stream(con);
    }

    fn open_stream(&self, connection: Connection) {
        let state = self.clone();
        tokio::spawn(async move {
            match connection.open_uni().await {
                Ok(stream) => {
                    state.packet_listeners.lock().await.push(stream);
                }
                Err(err) => {
                    eprintln!("{}", err);
                    return;
                }
            }
        });
    }

    fn accept_stream(&self, connection: Connection) {
        let state = self.clone();
        tokio::spawn(async move {
            let mut stream = match connection.accept_uni().await {
                Ok(stream) => stream,
                Err(err) => {
                    eprintln!("{}", err);
                    return;
                }
            };
            let mut buf = [0u8; 4096];
            while let Ok(size) = stream.read(&mut buf).await {
                if let Some(size) = size {
                    if let Err(err) = state.tun_write.lock().await.write(&buf[..size]).await {
                        eprintln!("{}", err);
                        return;
                    }
                }
            }
        });
    }

    fn accept_incoming_connections(&self) {
        let state = self.clone();
        tokio::spawn(async move {
            while let Some(connecting) = state.endpoint.accept().await {
                match connecting.await {
                    Ok(con) => {
                        state.add_connection(con);
                    }
                    Err(err) => {
                        eprintln!("{}", err);
                        return;
                    }
                }
            }
        });
    }

    fn send_outcoming_packets(&self, mut tun_read: ReadHalf<tun::AsyncDevice>) {
        let state = self.clone();
        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            while let Ok(size) = tun_read.read(&mut buf).await {
                let mut new_listeners_list: Vec<SendStream> = Vec::new();
                let mut lock = state.packet_listeners.lock().await;
                while let Some(mut listener) = lock.pop() {
                    if let Ok(_) = listener.write(&buf[..size]).await {
                        new_listeners_list.push(listener);
                    }
                }
                *lock = new_listeners_list;
            }
        });
    }

    fn add_peer_from_address(&self, address: String) {
        let state = self.clone();
        tokio::spawn(async move {
            let parsed_address = match parse_peer_addr(&address) {
                Ok(val) => val,
                Err(err) => {
                    eprintln!("{}", err);
                    return;
                }
            };
            let connection = match state.endpoint.connect(parsed_address, P2PTUN_ALPN).await {
                Ok(val) => val,
                Err(err) => {
                    eprintln!("{}", err);
                    return;
                }
            };
            state.add_connection(connection);
        });
    }

    fn get_addr(&self) -> Pin<Box<dyn Future<Output = anyhow::Result<PeerAddr>>>> {
        let state = self.clone();
        Box::pin(async move { state.endpoint.my_addr().await })
    }
}
