/// Enum representing errors that can happen in p2ptun's daemon
#[derive(Debug)]
pub enum DaemonError {
    TunError(tun::Error),
    AnyhowError(anyhow::Error),
    IoError(std::io::Error),
    TicketError(iroh_base::ticket::Error),
    KeyParsingError(iroh_net::key::KeyParsingError),
    Died,
}

impl From<tun::Error> for DaemonError {
    fn from(error: tun::Error) -> Self {
        Self::TunError(error)
    }
}

impl From<anyhow::Error> for DaemonError {
    fn from(error: anyhow::Error) -> Self {
        Self::AnyhowError(error)
    }
}

impl From<std::io::Error> for DaemonError {
    fn from(error: std::io::Error) -> Self {
        Self::IoError(error)
    }
}

impl From<iroh_base::ticket::Error> for DaemonError {
    fn from(error: iroh_base::ticket::Error) -> Self {
        Self::TicketError(error)
    }
}

impl From<iroh_net::key::KeyParsingError> for DaemonError {
    fn from(error: iroh_net::key::KeyParsingError) -> Self {
        Self::KeyParsingError(error)
    }
}
