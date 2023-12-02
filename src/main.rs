mod application;
mod constants;
mod peer_addr;

use std::{io::stdin, net::Ipv4Addr};

use application::{Application, ApplicationState};
use iroh_net::key::SecretKey;
use peer_addr::dump_peer_addr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let secret_key = match std::env::var("P2PTUN_SECRET_KEY") {
        Ok(key) => key.parse::<SecretKey>()?,
        Err(_) => SecretKey::generate(),
    };
    let netmask = match std::env::var("P2PTUN_NETMASK") {
        Ok(addr) => addr.parse::<Ipv4Addr>()?,
        Err(err) => {
            eprintln!("{}", err);
            Ipv4Addr::new(255, 0, 0, 0)
        }
    };
    let ip_address = match std::env::var("P2PTUN_IP_ADDR") {
        Ok(addr) => addr.parse::<Ipv4Addr>()?,
        Err(err) => {
            eprintln!("{}", err);
            Ipv4Addr::new(10, 0, 0, 1)
        }
    };

    let tun_config = {
        let mut tun_config = tun::configure();
        tun_config.up();
        tun_config.netmask(netmask);
        tun_config.address(ip_address);
        tun_config
    };

    let state = ApplicationState::start_application(secret_key, tun_config).await?;
    let my_addr = dump_peer_addr(&state.get_addr().await?);
    println!("{}", my_addr);

    loop {
        let mut buf = String::new();
        stdin().read_line(&mut buf)?;
        state.add_peer_from_address(buf);
    }
}
