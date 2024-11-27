use std::{collections::HashMap, net::{Ipv4Addr, SocketAddr}};

use serde::{Serialize, Deserialize};
use tokio::{sync::oneshot::Sender, time::Instant};

pub mod protos {
    tonic::include_proto!("consensus");
}
pub mod consensus_server;
pub mod malicious_consensus_server;
pub mod consensus_client;


#[derive(Debug)]
pub struct ConsensusMessage {
    pub nc_list: Vec<String>, 
    pub dev_addr: String, 
    pub packet: Vec<u8>, 
    pub rssi: i32,
    pub response: Sender<bool>
}

#[derive(PartialEq, Debug)]
pub enum ConsensusState {
    ReceivingDisseminations,
    ReceivingSets,
    END
}

#[derive(Debug)]
pub enum ConsensusError {
    NoRound,
    InvalidHash,
    InvalidHashLength,
    NotPartOfRound,
    NCAlreadyInSet,
    InvalidSet,
    WrongState,
    RoundEndedAlready,

    InvalidUri,
    InvalidCertificate,
    InvalidTlsConfig,
    Transport(tonic::transport::Error),
    IoError(std::io::Error),
}

impl PartialEq for ConsensusError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Transport(_l0), Self::Transport(_r0)) => false,//l0 == r0,
            (Self::IoError(_l0), Self::IoError(_r0)) => false,//l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl From<tonic::transport::Error> for ConsensusError {
    fn from(e: tonic::transport::Error) -> Self {
        ConsensusError::Transport(e)
    }
}

impl From<std::io::Error> for ConsensusError {
    fn from(e: std::io::Error) -> Self {
        ConsensusError::IoError(e)
    }
}

#[derive(Debug)]
pub struct ConsensusRound {
    pub started_at: Instant,
    pub status: ConsensusState,
    pub nc_list: Vec<String>,
    pub packet: Vec<u8>,
    pub nc_set: HashMap<String, i32>,
    pub received_sets: Vec<String>,
    pub sender: Sender<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusCerts {
    pub cert_path: String,
    pub key_path: String,
    pub ca_cert_path: String,
}

pub fn get_addr(id: &str) -> SocketAddr {
    let peer_part: &str = id.split('.').next().unwrap();
    let peer_number: u16 = peer_part[4..].parse().unwrap(); // 0
    SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5050 + peer_number)
}