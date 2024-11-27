
use std::ops::{Deref, DerefMut};
use std::fmt::Debug;
use std::time::Duration;
use blockchain_api::BlockchainClient;
use blockchain_api::exec_bridge::BlockchainExeClient;

use lorawan::{device::Device, utils::eui::EUI64};

use crate::communicator::{CommunicatorError, LoRaWANCommunicator, ReceivedTransmission};
use crate::configs::RadioDeviceConfig;
use crate::devices::lorawan_device::LoRaWANDevice;
use crate::split_communicator::{LoRaReceiver, LoRaSender, SplitCommunicator};

pub struct RadioDevice {
    device: LoRaWANDevice<RadioCommunicator>,
}

impl RadioDevice  {
    pub async fn create(device: Device, config: &RadioDeviceConfig) -> LoRaWANDevice<RadioCommunicator> {
        LoRaWANDevice::new(device, RadioCommunicator::from_config(config).await.unwrap())
    }

    pub async fn from_blockchain(dev_eui: &EUI64, config: &RadioDeviceConfig) -> LoRaWANDevice<RadioCommunicator> {
        let client = BlockchainExeClient::new("orderer1.orderers.dlwan.phd:6050", "lorawan", "lorawan", None);
        let device = client.get_device(dev_eui).await.unwrap();

        LoRaWANDevice::new(device, RadioCommunicator::from_config(config).await.unwrap())
    }
}

impl Deref for RadioDevice {
    type Target=LoRaWANDevice<RadioCommunicator>;

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

#[derive(Clone, Copy)]
pub struct RadioCommunicator {
    pub config: RadioDeviceConfig,
}

impl LoRaWANCommunicator for RadioCommunicator {
    type Config = RadioDeviceConfig;

    async fn from_config(config: &Self::Config) -> Result<Self, CommunicatorError> {
        Ok(Self { config: *config })
    }

    async fn send(
        &self,
        _bytes: &[u8],
        _src: Option<EUI64>,
        _dest: Option<EUI64>,
    ) -> Result<(), CommunicatorError> {
        todo!()
    }
    
    async fn receive(
        &self,
        _timeout: Option<Duration>,
    ) -> Result<Vec<ReceivedTransmission>, CommunicatorError> {
        todo!()
    }
}

pub struct RadioSender {
    _config: RadioDeviceConfig,
    _inner: u8,
}

impl LoRaSender for RadioSender {
    type OptionalInfo = ();

    async fn send(&self, _bytes: &[u8], _optional_info: Option<Self::OptionalInfo>) -> Result<(), CommunicatorError> {
        todo!()
    }
    
}

pub struct RadioReceiver {
    _config: RadioDeviceConfig,
    _inner: u8,
}

impl LoRaReceiver for RadioReceiver {
    async fn receive(&self, _timeout: Option<Duration>) -> Result<Vec<ReceivedTransmission>, CommunicatorError> {
        todo!()
    }
}

impl SplitCommunicator for RadioCommunicator {
    type Sender=RadioSender;
    type Receiver=RadioReceiver;

    async fn split_communicator(self) -> Result<(Self::Sender, Self::Receiver), CommunicatorError> {
        todo!()
    }
}