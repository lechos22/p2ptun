use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use iroh_net::{key::SecretKey, MagicEndpoint, PeerAddr};
use quinn::{Connection, ReadError, RecvStream, SendStream, WriteError};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    sync::Mutex,
};

use crate::{constants::P2PTUN_ALPN, peer_addr::parse_peer_addr};

pub struct ApplicationState {
    tun_write: Mutex<WriteHalf<tun::AsyncDevice>>,
    packet_listeners: Mutex<Vec<SendStream>>,
    endpoint: MagicEndpoint,
}

fn create_async_tun(
    config: tun::Configuration,
) -> anyhow::Result<(ReadHalf<tun::AsyncDevice>, WriteHalf<tun::AsyncDevice>)> {
    Ok(tokio::io::split(tun::create_as_async(&config)?))
}

impl ApplicationState {
    pub fn start_application(
        secret_key: SecretKey,
        tun_config: tun::Configuration,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Arc<Self>>>>> {
        Box::pin(async move {
            let endpoint = MagicEndpoint::builder()
                .secret_key(secret_key)
                .alpns(vec![P2PTUN_ALPN.to_vec()])
                .bind(0)
                .await?;
            while endpoint.my_derp().is_none() {
                tokio::time::sleep(Duration::from_millis(500)).await;
                eprintln!("Waiting for DERP...");
            }
            let (tun_read, tun_write) = create_async_tun(tun_config)?;
            let state = Arc::new(Self {
                tun_write: Mutex::new(tun_write),
                packet_listeners: Default::default(),
                endpoint,
            });

            state.accept_incoming_connections();
            state.send_outcoming_packets(tun_read);

            Ok(state)
        })
    }

    async fn handle_streams(
        &self,
        connection: Connection,
        (send, mut recv): (SendStream, RecvStream),
    ) {
        self.packet_listeners.lock().await.push(send);
        let mut buf = [0u8; 4096];
        loop {
            match recv.read(&mut buf).await {
                Ok(None)
                | Err(ReadError::ConnectionLost(_))
                | Err(ReadError::Reset(_))
                | Err(ReadError::UnknownStream) => {
                    break;
                }
                Ok(Some(size)) => {
                    eprintln!("Received {} bytes from {}", size, connection.stable_id());
                    if let Err(err) = self.tun_write.lock().await.write(&buf[..size]).await {
                        eprintln!("{}", err);
                    }
                }
                _ => {}
            }
        }
    }

    fn open_stream(self: Arc<ApplicationState>, connection: Connection) {
        tokio::spawn(async move {
            match connection.open_bi().await {
                Ok(streams) => self.handle_streams(connection, streams).await,
                Err(err) => {
                    eprintln!("{}", err);
                    return;
                }
            }
        });
    }

    fn accept_stream(self: Arc<ApplicationState>, connection: Connection) {
        tokio::spawn(async move {
            match connection.accept_bi().await {
                Ok(streams) => self.handle_streams(connection, streams).await,
                Err(err) => {
                    eprintln!("{}", err);
                    return;
                }
            };
        });
    }
}

pub trait Application {
    fn accept_incoming_connections(&self);
    fn send_outcoming_packets(&self, tun_read: ReadHalf<tun::AsyncDevice>);
    fn get_addr(&self) -> Pin<Box<dyn Future<Output = anyhow::Result<PeerAddr>>>>;
    fn add_peer_from_address(&self, address: String);
}

impl Application for Arc<ApplicationState> {
    fn accept_incoming_connections(&self) {
        let state = self.clone();
        tokio::spawn(async move {
            while let Some(connecting) = state.endpoint.accept().await {
                match connecting.await {
                    Ok(connection) => {
                        eprintln!("Connected to {}", connection.stable_id());
                        state.clone().accept_stream(connection);
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
                eprintln!("Sending {} bytes", size);
                let mut new_listeners_list: Vec<SendStream> = Vec::new();
                let mut lock = state.packet_listeners.lock().await;
                while let Some(mut listener) = lock.pop() {
                    match listener.write(&buf[..size]).await {
                        Ok(0)
                        | Err(WriteError::UnknownStream)
                        | Err(WriteError::ConnectionLost(_)) => {}
                        _ => {
                            new_listeners_list.push(listener);
                        }
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
            eprintln!("Connected to {}", connection.stable_id());
            state.open_stream(connection);
        });
    }

    fn get_addr(&self) -> Pin<Box<dyn Future<Output = anyhow::Result<PeerAddr>>>> {
        let state = self.clone();
        Box::pin(async move { state.endpoint.my_addr().await })
    }
}
