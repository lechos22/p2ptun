use std::env::args;

use p2ptun::{
    daemon::{run_daemon, DaemonConfig},
    remote::{dial_peer, disconnect_peer},
};

#[tokio::main]
async fn main() {
    let args: Vec<String> = args().skip(1).collect();
    let args_ref: Vec<&str> = args.iter().map(String::as_str).collect();
    match &args_ref[..] {
        &["daemon"] => {
            run_daemon(DaemonConfig::default()).await.unwrap();
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
