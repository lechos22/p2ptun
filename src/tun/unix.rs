use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use tokio::{sync::Mutex, task::JoinHandle};
use tun_tap::Iface;

use crate::errors::WrapErrors;

pub struct Tun {
    iface: Arc<Iface>,
    listener: Mutex<Option<JoinHandle<()>>>,
}

type PinnedThreadSafeFuture<T> = Pin<Box<dyn Future<Output = T> + Sync + Send>>;
type OnMessageFunction = Box<dyn Fn(Vec<u8>) -> PinnedThreadSafeFuture<()> + Sync + Send>;

impl Tun {
    pub fn new(ifname: &str) -> Result<Self, String> {
        let iface = Iface::new(ifname, tun_tap::Mode::Tun).wrap_errors()?;
        iface.set_non_blocking().wrap_errors()?;
        Ok(Tun {
            iface: Arc::new(iface),
            listener: Mutex::new(None),
        })
    }
    pub fn send(&self, packet: &[u8]) -> Result<usize, String> {
        self.iface.send(packet).wrap_errors()
    }
    pub async fn listen(&self, on_message: OnMessageFunction) {
        self.unlisten().await;
        let iface = self.iface.clone();
        *self.listener.lock().await = Some(tokio::spawn(async move {
            let mut buf: [u8; 1542] = [0; 1542];
            loop {
                match iface.recv(&mut buf) {
                    Ok(size) => {
                        on_message(buf[..size].to_vec()).await;
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(err) => {
                        eprintln!("TUN error {}", err);
                    }
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }));
    }
    pub async fn unlisten(&self) {
        let listener = self.listener.lock().await.take();
        if let Some(listener) = listener {
            listener.abort();
        }
    }
}
