use std::time::Duration;

use blockchain_api::{exec_bridge::BlockchainExeClient, BlockchainClient};
use lorawan::{device::Device, utils::eui::EUI64};
use crate::{communicator::{CommunicatorError, LoRaWANCommunicator, ReceivedTransmission, Transmission}, split_communicator::{LoRaReceiver, LoRaSender, SplitCommunicator}, configs::MockDeviceConfig, devices::lorawan_device::LoRaWANDevice};

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

impl LoRaWANCommunicator for MockCommunicator {
    type Config = MockDeviceConfig;

    async fn from_config(_config: &Self::Config) -> Result<Self, CommunicatorError> {
        Ok(Self)
    }


    async fn send(
        &self,
        _bytes: &[u8],
        _src: Option<EUI64>,
        _dest: Option<EUI64>,
    ) -> Result<(), CommunicatorError> {
        Ok(())
    }
    
    async fn receive(
        &self,
        _timeout: Option<Duration>,
    ) -> Result<Vec<ReceivedTransmission>, CommunicatorError> {
        Ok(vec![ReceivedTransmission { 
            transmission: Transmission {
                payload: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
                ..Default::default()
            }, 
            arrival_stats: Default::default()
        }])
    }  
}


#[derive(Default)]
pub struct MockSender;

#[derive(Default)]
pub struct MockReceiver;

impl LoRaSender for MockSender {
    type OptionalInfo = ();
    async fn send(&self, _bytes: &[u8], _optional_info: Option<Self::OptionalInfo>) -> Result<(), CommunicatorError> {
        Ok(())
    }
}

impl LoRaReceiver for MockReceiver {
    async fn receive(&self, _timeout: Option<Duration>) -> Result<Vec<ReceivedTransmission>, CommunicatorError> {
        Ok(vec![ReceivedTransmission { 
            transmission: Transmission {
                payload: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
                ..Default::default()
            }, 
            arrival_stats: Default::default()
        }])
    }
}

impl SplitCommunicator for MockCommunicator {
    type Sender=MockSender;
    type Receiver=MockReceiver;

    async fn split_communicator(self) -> Result<(Self::Sender, Self::Receiver), CommunicatorError> {
        Ok((MockSender, MockReceiver))
    }
    
}