use p2ptun::daemon::{run_daemon, DaemonConfig};

#[tokio::main]
async fn main() {
    run_daemon(DaemonConfig::default()).await.unwrap();
}
