use std::{time::Duration, collections::HashMap};

use async_trait::async_trait;
use blockchain_api::{exec_bridge::BlockchainExeClient, BlockchainClient};
use lorawan::{device::Device, utils::eui::EUI64, physical_parameters::SpreadingFactor};
use tokio::{net::TcpStream, io::{AsyncWriteExt, AsyncReadExt}, sync::Mutex};

use crate::{devices::lorawan_device::LoRaWANDevice, communicator::{LoRaWANCommunicator, CommunicatorError, LoRaPacket}, configs::TcpDeviceConfig};


pub struct TcpDevice;
impl TcpDevice {
    pub async fn create(device: Device, config: &TcpDeviceConfig) -> LoRaWANDevice<TCPCommunicator> {
        LoRaWANDevice::new(
            device,
            TCPCommunicator::from(TcpStream::connect(format!("{}:{}", config.addr, config.port)).await.unwrap()),
            //DeviceConfig { configuration: device, dtype: DeviceConfigType::TCP(TcpDeviceConfig { addr, port })  }
        )
    }

    pub async fn from_blockchain(dev_eui: &EUI64,config: &TcpDeviceConfig) -> LoRaWANDevice<TCPCommunicator> {
        let client = BlockchainExeClient::new("orderer1.orderers.dlwan.phd:6050", "lorawan", "lorawan", None);
        let device = client.get_device(dev_eui).await.unwrap();
        LoRaWANDevice::new(device, TCPCommunicator::from(TcpStream::connect(format!("{}:{}", config.addr, config.port)).await.unwrap()))
        /*DeviceConfig { configuration: device, dtype: DeviceConfigType::TCP(TcpDeviceConfig { addr, port })}*/
    }
}

pub struct TCPCommunicator {
    stream: Mutex<TcpStream>,
}

impl From<TcpStream> for TCPCommunicator {
    fn from(stream: TcpStream) -> Self {
        Self { stream: Mutex::new(stream) }
    }
}

#[async_trait]
impl LoRaWANCommunicator for TCPCommunicator {
    type Config = TcpDeviceConfig;

    async fn from_config(config: &Self::Config) -> Result<Self, CommunicatorError> {
        let stream = TcpStream::connect(format!("{}:{}", config.addr, config.port)).await.unwrap();
        Ok(Self { stream: Mutex::new(stream) })
    }

    async fn send_uplink(
        &self,
        bytes: &[u8],
        _src: Option<EUI64>,
        _dest: Option<EUI64>,
    ) -> Result<(), CommunicatorError> {
        let mut stream = self.stream.lock().await;
        Ok(stream.write_all(bytes).await?)
    }

    async fn receive_downlink(
        &self,
        _timeout: Option<Duration>,
    ) -> Result<HashMap<SpreadingFactor, LoRaPacket>, CommunicatorError> {
        let mut buf = Vec::with_capacity(256);
        let mut stream = self.stream.lock().await;
        let _len = stream.read_buf(&mut buf).await?;

        let packet = LoRaPacket {
            payload: buf,
            ..Default::default()
        };
        Ok(HashMap::from([(SpreadingFactor::new(7), packet)]))
    }
}