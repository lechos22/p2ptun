#![feature(never_type)]

use std::{
    collections::HashSet,
    io::{stdin, stdout, Write},
    sync::Arc,
};

use anyhow::anyhow;
use iroh_net::{key::SecretKey, AddrInfo, MagicEndpoint, PeerAddr};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{
        mpsc::{self, Sender},
        Mutex,
    },
};

const P2PTUN_ALPN: &[u8] = b"p2ptun";

async fn accept_connection(
    endpoint: &MagicEndpoint,
    incoming_packets_sender: Sender<Vec<u8>>,
) -> anyhow::Result<Sender<Vec<u8>>> {
    if let Some(incoming_connection) = endpoint.accept().await {
        let connection = incoming_connection.await?;
        let (mut send_stream, mut recv_stream) = connection.accept_bi().await?;
        let mut outcoming_packets = mpsc::channel::<Vec<u8>>(16);

        // Incoming packet router
        tokio::spawn(async move {
            let mut buffer = [0; 4096];
            while let Ok(Some(packet_size)) = recv_stream.read(&mut buffer).await {
                if let Err(_) = incoming_packets_sender
                    .send(buffer[..packet_size].to_vec())
                    .await
                {
                    return;
                }
            }
        });

        // Outcoming packet sender
        tokio::spawn(async move {
            while let Some(packet) = outcoming_packets.1.recv().await {
                if let Err(_) = send_stream.write(&packet).await {
                    return;
                }
            }
        });
        Ok(outcoming_packets.0)
    } else {
        Err(anyhow!("MagicEndpoint is closed"))
    }
}

async fn run(
    tun_config: tun::Configuration,
    secret_key: SecretKey,
) -> anyhow::Result<MagicEndpoint> {
    let (mut tun_read, mut tun_write) = tokio::io::split(tun::create_as_async(&tun_config)?);

    let endpoint = MagicEndpoint::builder()
        .secret_key(secret_key)
        .alpns(vec![P2PTUN_ALPN.to_vec()])
        .bind(0)
        .await?;

    while endpoint.my_derp().await.is_none() {}

    let mut incoming_packets = mpsc::channel::<Vec<u8>>(16);

    // Incoming packet sender
    tokio::spawn(async move {
        while let Some(packet) = incoming_packets.1.recv().await {
            if let Err(error) = tun_write.write(&packet).await {
                eprintln!("Error: {:?}", error);
            }
        }
    });

    let outcoming_packets_senders = Arc::new(Mutex::new(Vec::new()));

    // Magic sock connection handler
    let endpoint_clone = endpoint.clone();
    let outcoming_packets_senders_clone = outcoming_packets_senders.clone();
    tokio::spawn(async move {
        loop {
            match accept_connection(&endpoint_clone, incoming_packets.0.clone()).await {
                Ok(outcoming_packets_sender) => {
                    outcoming_packets_senders_clone
                        .lock()
                        .await
                        .push(outcoming_packets_sender);
                }
                Err(error) => {
                    eprintln!("Error: {:?}", error);
                }
            }
        }
    });

    // Outcoming packets router
    tokio::spawn(async move {
        let mut buffer = [0; 4096];
        loop {
            while let Ok(packet_size) = tun_read.read(&mut buffer).await {
                let mut run_gc = false;
                for outcoming_packets_sender in outcoming_packets_senders.lock().await.iter() {
                    if let Err(_) = outcoming_packets_sender
                        .send(buffer[..packet_size].to_vec())
                        .await
                    {
                        run_gc = true;
                    }
                }
                if run_gc {
                    outcoming_packets_senders
                        .lock()
                        .await
                        .retain(|sender| !sender.is_closed());
                }
            }
        }
    });

    Ok(endpoint)
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
async fn main() -> anyhow::Result<!> {
    let secret_key = match std::env::var("P2PTUN_SECRET_KEY") {
        Ok(key) => key.parse::<SecretKey>()?,
        Err(_) => {
            let secret_key = SecretKey::generate();
            secret_key
        }
    };

    let mut tun_config = tun::configure();
    tun_config.up();

    let endpoint = run(tun_config, secret_key).await?;

    println!(
        "Your address: {}",
        dump_peer_addr(&endpoint.my_addr().await?)
    );

    let mut buffer = String::with_capacity(64);

    loop {
        print!("$$$ ");
        stdout().flush()?;
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
                endpoint.add_peer_addr(parsed_address).await?;
            }
            _ => {}
        }
    }

    // pending::<!>().await;

    // unreachable!("Unresolvable future resolved");
}
