use std::net::IpAddr;
use std::ops::{Deref, DerefMut};
use std::fmt::Debug;
use blockchain_api::BlockchainClient;
use blockchain_api::exec_bridge::BlockchainExeClient;
use lorawan::{device::Device, utils::eui::EUI64};

use crate::communicators::ColosseumCommunication;
use crate::configs::{RadioDeviceConfig, DeviceConfig, DeviceConfigType, ColosseumDeviceConfig};
use crate::lorawan_device::LoRaWANDevice;

pub struct ColosseumDevice {
    device: LoRaWANDevice<ColosseumCommunication>,
}

impl ColosseumDevice {
    pub fn create(device: Device, ip_addr: IpAddr, radio_config: RadioDeviceConfig, sdr_lora_code: &'static str) -> LoRaWANDevice<ColosseumCommunication> {
        LoRaWANDevice::new(device, ColosseumCommunication::new(ip_addr, radio_config, sdr_lora_code), DeviceConfig { configuration: device, dtype: DeviceConfigType::COLOSSEUM( ColosseumDeviceConfig { radio_config, address: ip_addr } )  })
    }

    pub async fn from_blockchain(dev_eui: &EUI64, ip_addr: IpAddr, radio_config: RadioDeviceConfig, sdr_lora_code: &'static str) -> LoRaWANDevice<ColosseumCommunication> {
        let client = BlockchainExeClient::new("orderer1.orderers.dlwan.phd:6050", "lorawan", "lorawan", None);
        let device = client.get_device(dev_eui).await.unwrap();

        LoRaWANDevice::new(device, ColosseumCommunication::new(ip_addr, radio_config, sdr_lora_code), DeviceConfig { configuration: device, dtype: DeviceConfigType::COLOSSEUM( ColosseumDeviceConfig { radio_config, address: ip_addr } )  })
    }
}

impl Deref for ColosseumDevice {
    type Target=LoRaWANDevice<ColosseumCommunication>;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl DerefMut for ColosseumDevice {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.device
    }
}

impl Debug for ColosseumDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ColosseumDevice").field("device", &self.device).finish()
    }
}