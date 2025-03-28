use std::{net::SocketAddr, sync::Arc, time::Duration};

use blockchain_api::{exec_bridge::BlockchainExeClient, BlockchainClient};
use lorawan::{device::Device, utils::eui::EUI64};
use tokio::net::UdpSocket;
use lorawan::utils::errors::LoRaWANError;

use crate::{
    communicator::{CommunicatorError, LoRaWANCommunicator, ReceivedTransmission, Transmission},
    configs::UDPDeviceConfig,
    devices::lorawan_device::LoRaWANDevice, split_communicator::{LoRaReceiver, LoRaSender, SplitCommunicator},
};

pub struct UDPDevice;
impl UDPDevice {
    pub async fn create(
        device: Device,
        config: &UDPDeviceConfig,
    ) -> LoRaWANDevice<UDPCommunicator> {

        let sock = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        sock.connect(format!("{}:{}", config.addr, config.port)).await.unwrap();
        LoRaWANDevice::new(device,UDPCommunicator::new(sock))
    }

    pub async fn from_blockchain(
        dev_eui: &EUI64,
        config: &UDPDeviceConfig,
    ) -> LoRaWANDevice<UDPCommunicator> {
        let client = BlockchainExeClient::new(
            "orderer1.orderers.dlwan.phd:6050",
            "lorawan",
            "lorawan",
            None,
        );
        let device = client.get_device(dev_eui).await.unwrap();
        let sock = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        sock.connect(format!("{}:{}", config.addr, config.port)).await.unwrap();
        LoRaWANDevice::new(device, UDPCommunicator::new(sock))
    }
}

pub struct UDPCommunicator {
    socket: UdpSocket,
}

impl UDPCommunicator {
    pub fn new(socket: UdpSocket) -> Self {
        Self {
            socket,
        }
    }
}

impl LoRaWANCommunicator for UDPCommunicator {
    type Config = UDPDeviceConfig;

    async fn from_config(config: &Self::Config) -> Result<Self, CommunicatorError> {
        let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        socket.connect(format!("{}:{}", config.addr, config.port)).await.unwrap();
        Ok(Self {
            socket,
        })
    }

    async fn send(
        &self,
        bytes: &[u8],
        _src: Option<EUI64>,
        _dest: Option<EUI64>,
    ) -> Result<(), CommunicatorError> {
        self.socket.send(bytes).await?;
        Ok(())
    }

    async fn receive(
        &self,
        timeout: Option<Duration>,
    ) -> Result<Vec<ReceivedTransmission>, CommunicatorError> {
        let mut buf = Vec::with_capacity(256);
        let sock = &self.socket;
        match timeout {
            Some(d) => {
                if tokio::time::timeout(d, sock.recv(&mut buf))
                    .await
                    .is_err()
                {
                    return Err(CommunicatorError::LoRaWANError(
                        LoRaWANError::MissingDownlink,
                    ));
                }
            }
            None => {
                sock.recv_buf(&mut buf).await?;
            }
        }

        let packet = ReceivedTransmission {
            transmission: Transmission {
                payload: buf,
                ..Default::default()
            },
            ..Default::default()
        };
        Ok(vec![packet])
    }
}

pub struct UDPSender {
    socket: Arc<UdpSocket>,
}

pub struct UDPReceiver {
    socket: Arc<UdpSocket>,
}

impl LoRaSender for UDPSender {
    type OptionalInfo=SocketAddr;

    async fn send(&self, bytes: &[u8], optional_info: Option<Self::OptionalInfo>) -> Result<(), CommunicatorError> {
        if let Some(addr) = optional_info {
            self.socket.send_to(bytes, addr).await?;
        } else {
            self.socket.send(bytes).await?;
        }
        Ok(())
    }
    
}

impl LoRaReceiver for UDPReceiver {
    async fn receive(&self, timeout: Option<Duration>) -> Result<Vec<ReceivedTransmission>, CommunicatorError> {
        let mut buf = Vec::with_capacity(256);
        let sock = &self.socket;
        match timeout {
            Some(d) => {
                if tokio::time::timeout(d, sock.recv(&mut buf))
                    .await
                    .is_err()
                {
                    return Err(CommunicatorError::LoRaWANError(
                        LoRaWANError::MissingDownlink,
                    ));
                }
            }
            None => {
                sock.recv_buf(&mut buf).await?;
            }
        }

        let packet = ReceivedTransmission {
            transmission: Transmission {
                payload: buf,
                ..Default::default()
            },
            ..Default::default()
        };
        Ok(vec![packet])
    }
}


impl SplitCommunicator for UDPCommunicator {
    type Sender=UDPSender;
    type Receiver=UDPReceiver;

    async fn split_communicator(self) -> Result<(Self::Sender, Self::Receiver), CommunicatorError> {
        let socket = Arc::new(self.socket);

        Ok((
            UDPSender {
                socket: socket.clone(),
            },
            UDPReceiver {
                socket: socket.clone(),
            }
        ))
    }
}

impl UDPSender {
    pub fn new(socket: Arc<UdpSocket>) -> Self {
        Self {
            socket,
        }
    }

    pub fn into_inner(self) -> Arc<UdpSocket> {
        self.socket
    }
}


impl UDPReceiver {
    pub fn new(socket: Arc<UdpSocket>) -> Self {
        Self {
            socket,
        }
    }

    pub fn into_inner(self) -> Arc<UdpSocket> {
        self.socket
    }
}