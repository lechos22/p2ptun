use std::{
    error::Error,
    io::Write,
    net::SocketAddr,
    net::TcpStream,
    net::{Ipv6Addr, SocketAddrV6},
};

const CONTROL_SOCKET_ADDRESS: SocketAddr =
    SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, 2233, 0, 0));

fn help() {
    todo!("help message");
}

fn extended_help() {
    todo!("extended help message");
}

fn add_peer(addr: &str) -> Result<(), Box<dyn Error>> {
    let mut stream = TcpStream::connect(CONTROL_SOCKET_ADDRESS)?;
    writeln!(stream, "ADD_PEER {}", addr)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let arg_refs: Vec<_> = args.iter().map(AsRef::as_ref).collect();
    match &arg_refs as &[&str] {
        &[] => {
            help();
        }
        &["help"] => {
            extended_help();
        }
        &["add-peer", addr] => {
            add_peer(addr)?;
        }
        _ => {
            help();
        }
    }
    Ok(())
}
