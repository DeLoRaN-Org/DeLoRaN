use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use blockchain_api::{exec_bridge::BlockchainExeClient, BlockchainClient};
use lorawan::{device::Device, utils::eui::EUI64, physical_parameters::SpreadingFactor};
use crate::{communicator::{LoRaWANCommunicator, CommunicatorError, LoRaPacket}, devices::lorawan_device::LoRaWANDevice, configs::MockDeviceConfig};

pub struct MockDevice;
impl MockDevice {
    pub async fn create(device: Device) -> LoRaWANDevice<MockCommunicator> {
        LoRaWANDevice::new( device,MockCommunicator)
    }

    pub async fn from_blockchain(dev_eui: &EUI64) -> LoRaWANDevice<MockCommunicator> {
        let client = BlockchainExeClient::new("orderer1.orderers.dlwan.phd:6050", "lorawan", "lorawan", None);
        let device = client.get_device(dev_eui).await.unwrap();
        LoRaWANDevice::new(device, MockCommunicator)
    }
}



pub struct MockCommunicator;

#[async_trait]
impl LoRaWANCommunicator for MockCommunicator {
    type Config = MockDeviceConfig;

    async fn from_config(_config: &Self::Config) -> Result<Self, CommunicatorError> {
        Ok(Self)
    }


    async fn send_uplink(
        &self,
        _bytes: &[u8],
        _src: Option<EUI64>,
        _dest: Option<EUI64>,
    ) -> Result<(), CommunicatorError> {
        Ok(())
    }
    
    async fn receive_downlink(
        &self,
        _timeout: Option<Duration>,
    ) -> Result<HashMap<SpreadingFactor, LoRaPacket>, CommunicatorError> {
        Ok(HashMap::from([(SpreadingFactor::new(7), LoRaPacket {
            payload: Vec::from([1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18]),
            ..Default::default()
        })]))
    }  
}