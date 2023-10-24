use blockchain_api::{exec_bridge::BlockchainExeClient, BlockchainClient};
use lorawan::{device::Device, utils::eui::EUI64};
use crate::{communicators::MockCommunicator, lorawan_device::LoRaWANDevice, configs::{DeviceConfig, DeviceConfigType}};

pub struct MockDevice;
impl MockDevice {
    pub async fn create(device: Device) -> LoRaWANDevice<MockCommunicator> {
        LoRaWANDevice::new(
            device,
            MockCommunicator,
            DeviceConfig {
                configuration: device,
                dtype: DeviceConfigType::MOCK,
            }
        )
    }

    pub async fn from_blockchain(dev_eui: &EUI64) -> LoRaWANDevice<MockCommunicator> {
        let client = BlockchainExeClient::new("orderer1.orderers.dlwan.phd:6050", "lorawan", "lorawan", None);
        let device = client.get_device(dev_eui).await.unwrap();
        LoRaWANDevice::new(
            device,
            MockCommunicator,
            DeviceConfig {
                configuration: device,
                dtype: DeviceConfigType::MOCK,
            }
        )
    }
}
