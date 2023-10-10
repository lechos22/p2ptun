use std::pin::Pin;

use futures::{Future, future::BoxFuture};

#[cfg(unix)]
mod unix;

type PinnedThreadSafeFuture<T> = Pin<Box<dyn Future<Output = T> + Sync + Send>>;
type OnMessageFunction = Box<dyn Fn(Vec<u8>) -> PinnedThreadSafeFuture<()> + Sync + Send>;

pub trait Tun {
    fn get_name(&self) -> &str;
    fn send(&self, packet: &[u8]) -> anyhow::Result<usize>;
    fn listen(&self, on_message: OnMessageFunction) -> BoxFuture<'static, ()>;
    fn unlisten(&self) -> BoxFuture<'static, ()>;
}

#[cfg(unix)]
pub fn create_interface(name: &str) -> anyhow::Result<Box<dyn Tun>> {
    Ok(Box::new(unix::UnixTun::new(name)?))
}
