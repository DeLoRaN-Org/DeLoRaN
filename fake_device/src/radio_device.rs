use std::ops::{Deref, DerefMut};
use std::fmt::Debug;
use blockchain_api::BlockchainClient;
use blockchain_api::exec_bridge::BlockchainExeClient;
use lorawan::{device::Device, utils::eui::EUI64};

use crate::configs::{RadioDeviceConfig, DeviceConfigType, DeviceConfig};
use crate::{communicators::RadioCommunication, lorawan_device::LoRaWANDevice};

pub struct RadioDevice {
    device: LoRaWANDevice<RadioCommunication>,
}

impl RadioDevice  {
    pub fn create(device: Device, config: RadioDeviceConfig) -> LoRaWANDevice<RadioCommunication> {
        LoRaWANDevice::new(device, RadioCommunication::new(config), DeviceConfig { configuration: device, dtype: DeviceConfigType::RADIO(config)  })
    }

    pub async fn from_blockchain(dev_eui: &EUI64, config: RadioDeviceConfig) -> LoRaWANDevice<RadioCommunication> {
        let client = BlockchainExeClient::new("orderer1.orderers.dlwan.phd:6050", "lorawan", "lorawan", None);
        let device = client.get_device(dev_eui).await.unwrap();

        LoRaWANDevice::new(device, RadioCommunication::new(config), DeviceConfig { configuration: device, dtype: DeviceConfigType::RADIO(config)  })
    }
}

impl Deref for RadioDevice {
    type Target=LoRaWANDevice<RadioCommunication>;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl DerefMut for RadioDevice {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.device
    }
}

impl Debug for RadioDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RadioDevice").field("device", &self.device).finish()
    }
}