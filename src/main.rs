use std::env::{args, var};

use p2ptun::{
    daemon::{actors::tun::TunConfig, run_daemon, DaemonConfig},
    remote::{dial_peer, disconnect_peer},
};

#[tokio::main]
async fn main() {
    let args: Vec<String> = args().skip(1).collect();
    let args_ref: Vec<&str> = args.iter().map(String::as_str).collect();
    match &args_ref[..] {
        &["daemon"] => {
            let tun_config = match (var("TUN_ADDRESS"), var("TUN_NETMASK")) {
                (Ok(tun_address), Ok(tun_netmask)) => Some(TunConfig {
                    ip: tun_address,
                    netmask: tun_netmask,
                }),
                _ => None,
            };
            run_daemon(DaemonConfig {
                tun: tun_config,
            })
            .await
            .unwrap();
        }
        &["ctl", "dial-peer", node_ticket] => {
            dial_peer(node_ticket).unwrap();
        }
        &["ctl", "disconnect-peer", node_id] => {
            disconnect_peer(node_id).unwrap();
        }
        _ => {}
    }
}
