use std::{collections::BTreeSet, sync::Arc};

use anyhow::anyhow;
use iroh_net::{key::SecretKey, AddrInfo, MagicEndpoint, PeerAddr};
use quinn::{Connection, SendStream};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    sync::Mutex,
};

const P2PTUN_ALPN: &[u8] = b"p2ptun";

#[inline]
fn duplicate<T: Clone>(el: T) -> (T, T) {
    (el.clone(), el)
}

async fn open_stream(
    connection: Connection,
    outcoming_packet_listeners: Arc<Mutex<Vec<SendStream>>>,
) {
    match connection.open_uni().await {
        Ok(stream) => {
            outcoming_packet_listeners.lock().await.push(stream);
        }
        Err(err) => {
            eprintln!("{}", err);
            return;
        }
    }
}

async fn accept_stream(connection: Connection, tun_write: Arc<Mutex<WriteHalf<tun::AsyncDevice>>>) {
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
            if let Err(err) = tun_write.lock().await.write(&buf[..size]).await {
                eprintln!("{}", err);
                return;
            }
        }
    }
}

async fn accept_incoming_connections(endpoint: MagicEndpoint, add_connection: impl Fn(Connection)) {
    while let Some(connecting) = endpoint.accept().await {
        match connecting.await {
            Ok(con) => {
                add_connection(con);
            }
            Err(err) => {
                eprintln!("{}", err);
                return;
            }
        }
    }
}

async fn send_outcoming_packets(
    mut tun_read: ReadHalf<tun::AsyncDevice>,
    listeners: Arc<Mutex<Vec<SendStream>>>,
) {
    let mut buf = [0u8; 4096];
    while let Ok(size) = tun_read.read(&mut buf).await {
        let mut new_listeners_list: Vec<SendStream> = Vec::new();
        let mut lock = listeners.lock().await;
        while let Some(mut listener) = lock.pop() {
            if let Ok(_) = listener.write(&buf[..size]).await {
                new_listeners_list.push(listener);
            }
        }
        *lock = new_listeners_list;
    }
}

fn create_connection_handler(
    outcoming_packet_listeners: Arc<Mutex<Vec<SendStream>>>,
    tun_write: WriteHalf<tun::AsyncDevice>,
) -> impl Fn(Connection) + Clone {
    let tun_write = Arc::new(Mutex::new(tun_write));
    move |con: Connection| {
        eprintln!("Connecting to {}", con.remote_address());
        let con = duplicate(con);
        tokio::spawn(open_stream(con.0, outcoming_packet_listeners.clone()));
        tokio::spawn(accept_stream(con.1, tun_write.clone()));
    }
}

fn create_async_tun(
    config: tun::Configuration,
) -> anyhow::Result<(ReadHalf<tun::AsyncDevice>, WriteHalf<tun::AsyncDevice>)> {
    Ok(tokio::io::split(tun::create_as_async(&config)?))
}

async fn run(
    tun_config: tun::Configuration,
    secret_key: SecretKey,
) -> anyhow::Result<(MagicEndpoint, impl Fn(quinn::Connection) + Clone)> {
    let (tun_read, tun_write) = create_async_tun(tun_config)?;
    let endpoint = MagicEndpoint::builder()
        .secret_key(secret_key)
        .alpns(vec![P2PTUN_ALPN.to_vec()])
        .bind(0)
        .await?;
    while endpoint.my_derp().await.is_none() { /* waiting for DERP in an ugly, but working way */ }
    let packet_listeners = duplicate(Arc::<Mutex<Vec<SendStream>>>::default());
    let add_connection = duplicate(create_connection_handler(packet_listeners.0, tun_write));
    let endpoint = duplicate(endpoint);
    tokio::spawn(accept_incoming_connections(endpoint.0, add_connection.0));
    tokio::spawn(send_outcoming_packets(tun_read, packet_listeners.1));
    Ok((endpoint.1, add_connection.1))
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
                .collect::<BTreeSet<_>>(),
        },
    })
}

async fn add_peer_from_address(
    address: String,
    endpoint: MagicEndpoint,
    add_connection: impl Fn(Connection),
) {
    let parsed_address = match parse_peer_addr(&address) {
        Ok(val) => val,
        Err(err) => {
            eprintln!("{}", err);
            return;
        }
    };
    let connection = match endpoint.connect(parsed_address, P2PTUN_ALPN).await {
        Ok(val) => val,
        Err(err) => {
            eprintln!("{}", err);
            return;
        }
    };
    add_connection(connection);
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
    let my_addr = dump_peer_addr(&endpoint.my_addr().await?);

    let window = MainWindow::new()?;
    window.set_peer_addr(my_addr.clone().into());
    window.on_copy_addr(move || {
        let mut clipboard = match arboard::Clipboard::new() {
            Ok(val) => val,
            Err(err) => {
                eprintln!("{}", err);
                return;
            }
        };
        let _ = clipboard.set_text(my_addr.clone());
    });
    window.on_add_peer(move |address| {
        tokio::spawn(add_peer_from_address(
            address.to_string(),
            endpoint.clone(),
            add_connection.clone(),
        ));
    });
    window.run()?;
    Ok(())
}

slint::slint! {
    import { Button , TextEdit, ScrollView} from "std-widgets.slint";
    export component MainWindow inherits Window {
        callback add_peer(string);
        callback copy_addr();
        in property <string> peer_addr;
        width: 800px;
        height: 600px;
        VerticalLayout {
            height: 20rem;
            width: 40rem;
            ScrollView {
                height: 3rem;
                viewport-width: t.width + 1rem;
                padding: 0.5rem;
                t := Text {
                    text: peer-addr;
                }
            }
            Button {
                text: "Copy address";
                height: 2rem;
                clicked => {
                    copy-addr();
                }
            }
            text_input := TextEdit {
                padding: 0.5rem;
                height: 3rem;
            }
            Button {
                text: "Add peer";
                height: 2rem;
                clicked => {
                    add-peer(text-input.text);
                }
            }
        }
    }
}
