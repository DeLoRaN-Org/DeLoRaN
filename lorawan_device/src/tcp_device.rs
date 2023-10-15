use blockchain_api::{exec_bridge::BlockchainExeClient, BlockchainClient};
use lorawan::{device::Device, utils::eui::EUI64};
use tokio::net::TcpStream;

use crate::{communicators::TCPCommunication, lorawan_device::LoRaWANDevice, configs::{DeviceConfigType, TcpDeviceConfig, DeviceConfig}};

pub struct TcpDevice;
impl TcpDevice {
    pub async fn create(device: Device, addr: String, port: u16) -> LoRaWANDevice<TCPCommunication> {
        LoRaWANDevice::new(
            device,
            TCPCommunication::from(TcpStream::connect(format!("{}:{}", addr, port)).await.unwrap()),
            DeviceConfig { configuration: device, dtype: DeviceConfigType::TCP(TcpDeviceConfig { addr, port })  }
        )
    }

    pub async fn from_blockchain(dev_eui: &EUI64,addr: String, port: u16) -> LoRaWANDevice<TCPCommunication> {
        let client = BlockchainExeClient::new("orderer1.orderers.dlwan.phd:6050", "lorawan", "lorawan", None);
        let device = client.get_device(dev_eui).await.unwrap();
        LoRaWANDevice::new(device, TCPCommunication::from(TcpStream::connect(format!("{}:{}", addr, port)).await.unwrap()), DeviceConfig { configuration: device, dtype: DeviceConfigType::TCP(TcpDeviceConfig { addr, port })})
    }
}
