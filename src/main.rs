use std::{collections::BTreeSet, future::Future, pin::Pin, sync::Arc};

use anyhow::anyhow;
use iroh_net::{key::SecretKey, AddrInfo, MagicEndpoint, PeerAddr};
use quinn::{Connection, SendStream};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    sync::Mutex,
};

const P2PTUN_ALPN: &[u8] = b"p2ptun";

struct ApplicationState {
    tun_write: Mutex<WriteHalf<tun::AsyncDevice>>,
    packet_listeners: Mutex<Vec<SendStream>>,
    endpoint: MagicEndpoint,
}

trait Application {
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
        let con = duplicate(con);
        self.open_stream(con.0);
        self.accept_stream(con.1);
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
        Box::pin(async move {
            state.endpoint.my_addr().await
        })
    }
}

#[inline]
fn duplicate<T: Clone>(el: T) -> (T, T) {
    (el.clone(), el)
}

fn create_async_tun(
    config: tun::Configuration,
) -> anyhow::Result<(ReadHalf<tun::AsyncDevice>, WriteHalf<tun::AsyncDevice>)> {
    Ok(tokio::io::split(tun::create_as_async(&config)?))
}

async fn run(
    tun_config: tun::Configuration,
    secret_key: SecretKey,
) -> anyhow::Result<impl Application + Clone> {
    let (tun_read, tun_write) = create_async_tun(tun_config)?;
    let endpoint = MagicEndpoint::builder()
        .secret_key(secret_key)
        .alpns(vec![P2PTUN_ALPN.to_vec()])
        .bind(0)
        .await?;
    while endpoint.my_derp().await.is_none() { /* waiting for DERP in an ugly, but working way */ }
    let state = Arc::new(ApplicationState {
        endpoint: endpoint,
        packet_listeners: Mutex::<Vec<SendStream>>::default(),
        tun_write: Mutex::new(tun_write),
    });
    state.accept_incoming_connections();
    state.send_outcoming_packets(tun_read);
    Ok(state)
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

    let state = run(tun_config, secret_key).await?;
    let my_addr = dump_peer_addr(&state.get_addr().await?);

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
        state.add_peer_from_address(address.to_string());
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
