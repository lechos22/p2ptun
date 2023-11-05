mod application;
mod constants;
mod peer_arddr;

use std::sync::Arc;

use application::{Application, ApplicationState};
use constants::P2PTUN_ALPN;
use iroh_net::{key::SecretKey, MagicEndpoint};
use peer_arddr::dump_peer_addr;
use tokio::io::{ReadHalf, WriteHalf};

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
    let state = Arc::new(ApplicationState::create(endpoint, tun_write));
    state.accept_incoming_connections();
    state.send_outcoming_packets(tun_read);
    Ok(state)
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
