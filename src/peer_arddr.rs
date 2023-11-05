use std::collections::BTreeSet;

use anyhow::anyhow;
use iroh_net::{AddrInfo, PeerAddr};

pub fn dump_peer_addr(peer_addr: &PeerAddr) -> String {
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

pub fn parse_peer_addr<'a>(text: &str) -> anyhow::Result<PeerAddr> {
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
