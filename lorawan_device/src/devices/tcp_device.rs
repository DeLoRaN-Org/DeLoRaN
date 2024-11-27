use std::time::Duration;

use blockchain_api::{exec_bridge::BlockchainExeClient, BlockchainClient};
use lorawan::{device::Device, utils::eui::EUI64};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{tcp::{OwnedReadHalf, OwnedWriteHalf}, TcpStream},
    sync::{Mutex, RwLock},
};

use lorawan::utils::errors::LoRaWANError;

use crate::{
    communicator::{CommunicatorError, LoRaWANCommunicator, ReceivedTransmission, Transmission},
    configs::TcpDeviceConfig,
    devices::lorawan_device::LoRaWANDevice, split_communicator::{LoRaReceiver, LoRaSender, SplitCommunicator},
};

pub struct TcpDevice;
impl TcpDevice {
    pub async fn create(
        device: Device,
        config: &TcpDeviceConfig,
    ) -> LoRaWANDevice<TCPCommunicator> {
        LoRaWANDevice::new(
            device,
            TCPCommunicator::from(
                TcpStream::connect(format!("{}:{}", config.addr, config.port))
                    .await
                    .unwrap(),
            ),
            //DeviceConfig { configuration: device, dtype: DeviceConfigType::TCP(TcpDeviceConfig { addr, port })  }
        )
    }

    pub async fn from_blockchain(
        dev_eui: &EUI64,
        config: &TcpDeviceConfig,
    ) -> LoRaWANDevice<TCPCommunicator> {
        let client = BlockchainExeClient::new(
            "orderer1.orderers.dlwan.phd:6050",
            "lorawan",
            "lorawan",
            None,
        );
        let device = client.get_device(dev_eui).await.unwrap();
        LoRaWANDevice::new(
            device,
            TCPCommunicator::from(
                TcpStream::connect(format!("{}:{}", config.addr, config.port))
                    .await
                    .unwrap(),
            ),
        )
        /*DeviceConfig { configuration: device, dtype: DeviceConfigType::TCP(TcpDeviceConfig { addr, port })}*/
    }
}

pub struct TCPCommunicator {
    stream: Mutex<TcpStream>,
}

impl From<TcpStream> for TCPCommunicator {
    fn from(stream: TcpStream) -> Self {
        Self {
            stream: Mutex::new(stream),
        }
    }
}

impl LoRaWANCommunicator for TCPCommunicator {
    type Config = TcpDeviceConfig;

    async fn from_config(config: &Self::Config) -> Result<Self, CommunicatorError> {
        let stream = TcpStream::connect(format!("{}:{}", config.addr, config.port))
            .await
            .unwrap();
        Ok(Self {
            stream: Mutex::new(stream),
        })
    }

    async fn send(
        &self,
        bytes: &[u8],
        _src: Option<EUI64>,
        _dest: Option<EUI64>,
    ) -> Result<(), CommunicatorError> {
        let mut stream = self.stream.lock().await;
        Ok(stream.write_all(bytes).await?)
    }

    async fn receive(
        &self,
        timeout: Option<Duration>,
    ) -> Result<Vec<ReceivedTransmission>, CommunicatorError> {
        let mut buf = Vec::with_capacity(256);
        let mut stream = self.stream.lock().await;
        match timeout {
            Some(d) => {
                if tokio::time::timeout(d, stream.read_buf(&mut buf))
                    .await
                    .is_err()
                {
                    return Err(CommunicatorError::LoRaWANError(
                        LoRaWANError::MissingDownlink,
                    ));
                }
            }
            None => {
                stream.read_buf(&mut buf).await?;
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


pub struct TcpSender {
    inner: RwLock<OwnedWriteHalf>,
}

impl LoRaSender for TcpSender {
    type OptionalInfo = ();
    async fn send(&self, bytes: &[u8], _optional_info: Option<Self::OptionalInfo>) -> Result<(), CommunicatorError> {
        self.inner.write().await.write_all(bytes).await?;
        Ok(())
    }
}

pub struct TcpReceiver {
    inner: RwLock<OwnedReadHalf>,
}

impl LoRaReceiver for TcpReceiver {
    async fn receive(&self, _timeout: Option<Duration>) -> Result<Vec<ReceivedTransmission>, CommunicatorError> {
        let mut buf = Vec::with_capacity(256);
        self.inner.write().await.read_buf(&mut buf).await?;
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

impl SplitCommunicator for TCPCommunicator {
    type Sender = TcpSender;
    type Receiver = TcpReceiver;

    async fn split_communicator(self) -> Result<(Self::Sender, Self::Receiver), CommunicatorError> {
        let (r, w) = {
            let stream = self.stream.into_inner();
            stream.into_split()
        };
        Ok((
            TcpSender {
                inner: RwLock::new(w),
            },
            TcpReceiver {
                inner: RwLock::new(r),
            },
        ))
    }
}