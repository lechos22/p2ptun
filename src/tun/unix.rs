use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use futures::{future::BoxFuture, FutureExt};
use log::error;
use tokio::{sync::Mutex, task::JoinHandle};
use tun_tap::Iface;

use super::Tun;

#[derive(Debug, Clone)]
pub struct UnixTun {
    iface: Arc<Iface>,
    listener: Arc<Mutex<Option<JoinHandle<()>>>>,
}

type PinnedThreadSafeFuture<T> = Pin<Box<dyn Future<Output = T> + Sync + Send>>;
type OnMessageFunction = Box<dyn Fn(Vec<u8>) -> PinnedThreadSafeFuture<()> + Sync + Send>;

impl UnixTun {
    pub fn new(ifname: &str) -> anyhow::Result<Self> {
        let iface = Iface::new(ifname, tun_tap::Mode::Tun)?;
        iface.set_non_blocking()?;
        Ok(UnixTun {
            iface: Arc::new(iface),
            listener: Arc::default(),
        })
    }
}

impl Tun for UnixTun {
    fn get_name(&self) -> &str {
        self.iface.name()
    }
    fn send(&self, packet: &[u8]) -> anyhow::Result<usize> {
        Ok(self.iface.send(packet)?)
    }
    fn listen(&self, on_message: OnMessageFunction) -> BoxFuture<'static, ()> {
        let self_clone = self.clone();
        async move {
            self_clone.unlisten().await;
            let iface = self_clone.iface.clone();
            *self_clone.listener.lock().await = Some(tokio::spawn(async move {
                let mut buf: [u8; 1542] = [0; 1542];
                loop {
                    match iface.recv(&mut buf) {
                        Ok(size) => {
                            on_message(buf[..size].to_vec()).await;
                        }
                        Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(err) => {
                            error!("TUN error {}", err);
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            }));
        }.boxed()
    }
    fn unlisten(&self) -> BoxFuture<'static, ()> {
        let self_clone = self.clone();
        async move {
            let listener = self_clone.listener.lock().await.take();
            if let Some(listener) = listener {
                listener.abort();
            }
        }.boxed()
    }
}
