use p2ptun::daemon::{Daemon, DaemonConfig};

#[tokio::main]
async fn main() {
    Daemon::new(DaemonConfig::default()).run().await;
}
