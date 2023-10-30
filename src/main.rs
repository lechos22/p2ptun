use std::{collections::HashSet, io::stdin, sync::Arc};

use anyhow::anyhow;
use iroh_net::{key::SecretKey, AddrInfo, MagicEndpoint, PeerAddr};
use quinn::{Connection, SendStream};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};

const P2PTUN_ALPN: &[u8] = b"p2ptun";

#[inline]
fn duplicate<T: Clone>(el: T) -> (T, T) {
    (el.clone(), el)
}

async fn run(
    tun_config: tun::Configuration,
    secret_key: SecretKey,
) -> anyhow::Result<(MagicEndpoint, impl Fn(quinn::Connection))> {
    let (mut tun_read, tun_write) = tokio::io::split(tun::create_as_async(&tun_config)?);

    let tun_write = Arc::new(Mutex::new(tun_write));
    let endpoint = MagicEndpoint::builder()
        .secret_key(secret_key)
        .alpns(vec![P2PTUN_ALPN.to_vec()])
        .bind(0)
        .await?;

    while endpoint.my_derp().await.is_none() { /* waiting for magic endpoint to work */ }

    let outcoming_packet_listeners: Arc<Mutex<Vec<SendStream>>> = Default::default();

    let add_connection = {
        let outcoming_packet_listeners = outcoming_packet_listeners.clone();
        move |con: Connection| {
            let (con_open, con_accept) = duplicate(con);
            let tun_write = tun_write.clone();
            let outcoming_packet_listeners = outcoming_packet_listeners.clone();
            tokio::spawn(async move {
                if let Ok(stream) = con_open.open_uni().await {
                    outcoming_packet_listeners.lock().await.push(stream);
                }
            });
            tokio::spawn(async move {
                if let Ok(mut stream) = con_accept.accept_uni().await {
                    let mut buf = [0u8; 4096];
                    while let Ok(size) = stream.read(&mut buf).await {
                        if let Some(size) = size {
                            let _ = tun_write.lock().await.write(&buf[..size]).await;
                        }
                    }
                }
            });
        }
    };

    {
        let endpoint = endpoint.clone();
        let add_connection = add_connection.clone();
        tokio::spawn(async move {
            while let Some(connecting) = endpoint.accept().await {
                println!("\nConnection from {}\n", connecting.remote_address());
                if let Ok(con) = connecting.await {
                    add_connection(con);
                }
            }
        });
    }

    tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        while let Ok(size) = tun_read.read(&mut buf).await {
            let mut new_listeners_list: Vec<SendStream> = Vec::new();
            let mut lock = outcoming_packet_listeners.lock().await;
            while let Some(mut listener) = lock.pop() {
                if let Ok(_) = listener.write(&buf[..size]).await {
                    new_listeners_list.push(listener);
                }
            }
            *lock = new_listeners_list;
        }
    });

    Ok((endpoint, add_connection))
}

fn dump_peer_addr(peer_addr: &PeerAddr) -> String {
    format!(
        "{};{};{}",
        peer_addr.peer_id,
        peer_addr
            .info
            .derp_region
            .map(|x| x.to_string())
            .unwrap_or("".to_string()),
        peer_addr
            .info
            .direct_addresses
            .iter()
            .map(|sock_addr| sock_addr.to_string())
            .collect::<Vec<_>>()
            .join(";")
    )
}

fn parse_peer_addr<'a>(text: &str) -> anyhow::Result<PeerAddr> {
    let mut split = text.split(";");
    let Some(peer_id) = split.next() else {
        return Err(anyhow!("Bad peer address"));
    };
    let derp_region = match split.next() {
        None => None,
        Some("") => None,
        Some(text) => Some(text.parse::<u16>()?),
    };
    Ok(PeerAddr {
        peer_id: peer_id.parse()?,
        info: AddrInfo {
            derp_region,
            direct_addresses: split
                .filter_map(|addr| addr.parse().ok())
                .collect::<HashSet<_>>(),
        },
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let secret_key = match std::env::var("P2PTUN_SECRET_KEY") {
        Ok(key) => key.parse::<SecretKey>()?,
        Err(_) => {
            let secret_key = SecretKey::generate();
            secret_key
        }
    };

    let mut tun_config = tun::configure();
    tun_config.up();

    let (endpoint, add_connection) = run(tun_config, secret_key).await?;

    println!(
        "Your address: {}",
        dump_peer_addr(&endpoint.my_addr().await?)
    );

    let mut buffer = String::with_capacity(64);

    loop {
        stdin().read_line(&mut buffer)?;
        let mut split = buffer.split(' ');
        match split.next() {
            Some("add-peer") => {
                let Some(address) = split.next() else {
                    continue;
                };
                let Ok(parsed_address) = parse_peer_addr(address) else {
                    continue;
                };
                let connection = endpoint.connect(parsed_address, P2PTUN_ALPN).await?;
                add_connection(connection);
            }
            _ => {}
        }
    }
}
