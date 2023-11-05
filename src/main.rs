mod application;
mod constants;
mod peer_arddr;

use application::{Application, ApplicationState};
use iroh_net::key::SecretKey;
use peer_arddr::dump_peer_addr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let secret_key = match std::env::var("P2PTUN_SECRET_KEY") {
        Ok(key) => key.parse::<SecretKey>()?,
        Err(_) => {
            let secret_key = SecretKey::generate();
            secret_key
        }
    };

    let tun_config = {
        let mut tun_config = tun::configure();
        tun_config.up();
        tun_config
    };

    let state = ApplicationState::start_application(secret_key, tun_config).await?;
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
