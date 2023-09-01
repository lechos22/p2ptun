use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use tokio::{sync::Mutex, task::JoinHandle};
use tun_tap::Iface;

use crate::errors::WrapErrors;

pub struct Tun {
    iface: Arc<Iface>,
    listener: Mutex<Option<JoinHandle<()>>>,
}

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
    pub fn listen(
        &self,
        on_message: Box<
            dyn Fn(Vec<u8>) -> Pin<Box<dyn Future<Output = ()> + Sync + Send>> + Sync + Send,
        >,
    ) -> () {
        self.unlisten();
        let iface = self.iface.clone();
        *self.listener.blocking_lock() = Some(tokio::spawn(async move {
            let mut buf: [u8; 1542] = [0; 1542];
            loop {
                match iface.recv(&mut buf) {
                    Ok(size) => {
                        on_message(buf[..size].to_vec()).await;
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(err) => {
                        eprintln!("TUN error {}", err.to_string());
                    }
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }));
    }
    pub fn unlisten(&self) -> () {
        let listener = self.listener.blocking_lock().take();
        if let Some(listener) = listener {
            listener.abort();
        }
    }
}
