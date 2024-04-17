//! # p2ptun: Peer-to-Peer Tunneling VPN
//! 
//! **p2ptun** is a program designed to establish Virtual Private Network (VPN) tunnels using peer-to-peer (P2P) connections. This tool allows users to securely connect different nodes or devices over the internet, creating a private network that encrypts and routes network traffic between peers.
//! 
//! ## Key Features
//! 
//! - **P2P Connectivity**: Utilizes peer-to-peer connections to establish secure tunnels between network nodes.
//! - **End-to-End Encryption**: Ensures data privacy and security by encrypting traffic transmitted over the VPN tunnels.
//! - **TUN Device Integration**: Interacts with TUN devices to read and write network packets, enabling seamless integration with the host system's networking stack.
//! - **Asynchronous I/O with Tokio**: Leverages Tokio's asynchronous runtime to handle concurrent network I/O operations efficiently.
//! 
//! ## How It Works
//! 1. **Node Discovery**: Nodes discover each other through a decentralized mechanism (Iroh's DERP-based relays), allowing them to establish direct P2P connections.
//! 2. **Tunnel Establishment**: Once peers are connected, **p2ptun** sets up encrypted tunnels between them and traffics network packets to TUN devices.
//! 3. **Secure Data Transfer**: Network traffic is encrypted at one end of the tunnel and decrypted at the other, ensuring end-to-end security.
//! 
//! ## Use Cases
//! 
//! - **Secure Remote Access**: Access private networks securely over the internet, ideal for remote work scenarios.
//! - **Decentralized Networking**: Build resilient and decentralized networks without relying on centralized infrastructure.
//! - **IoT and Edge Computing**: Connect IoT devices securely and enable edge computing applications with protected communication channels.
//! 
//! **p2ptun** empowers users to establish private and secure communication channels over the internet using P2P technology, offering flexibility and resilience for various networking applications.
//! 
//! ## Warning
//! 
//! This solution does **not** give you anonymity, as it is not what p2ptun is about.
//! 
//! Also, it is highly experimental and you use it at your own risk.

pub mod daemon;
