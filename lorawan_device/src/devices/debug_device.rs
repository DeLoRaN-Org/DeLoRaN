use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    time::{Duration, SystemTime},
};

use crate::{
    communicator::{CommunicatorError, LoRaPacket, LoRaWANCommunicator},
    devices::lorawan_device::LoRaWANDevice,
};
use async_trait::async_trait;
use blockchain_api::{exec_bridge::BlockchainExeClient, BlockchainClient};
use lorawan::{
    device::Device,
    physical_parameters::SpreadingFactor,
    utils::{eui::EUI64, PrettyHexSlice},
};

pub struct DebugDevice;
impl DebugDevice {
    pub fn create<T: LoRaWANCommunicator>(
        device: Device,
        communicator: T,
    ) -> LoRaWANDevice<DebugCommunicator<T>> {
        LoRaWANDevice::new(device, DebugCommunicator {
            communicator,
            id: None
        })
    }

    pub async fn from_blockchain<T: LoRaWANCommunicator>(
        dev_eui: &EUI64,
        communicator: T,
    ) -> LoRaWANDevice<DebugCommunicator<T>> {
        let client = BlockchainExeClient::new(
            "orderer1.orderers.dlwan.phd:6050",
            "lorawan",
            "lorawan",
            None,
        );
        let device = client.get_device(dev_eui).await.unwrap();
        LoRaWANDevice::new(device, DebugCommunicator {
            communicator,
            id: None
        })
    }

    pub fn from<T: LoRaWANCommunicator + Send + Sync>(d: LoRaWANDevice<T>) -> LoRaWANDevice<DebugCommunicator<T>> {
        let (device, communicator) = d.into();
        let id = Some(*device.dev_eui());
        LoRaWANDevice::new(device, DebugCommunicator {
            communicator,
            id
        })
    }
}

pub struct DebugCommunicator<T: LoRaWANCommunicator> {
    communicator: T,
    id: Option<EUI64>
}

impl <T: LoRaWANCommunicator> DebugCommunicator<T> {
    pub fn set_id(&mut self, id: &EUI64) {
        self.id = Some(*id)
    }

    pub fn from(c: T, id: Option<EUI64>) -> DebugCommunicator<T> where T: LoRaWANCommunicator + Send + Sync {
        DebugCommunicator {
            communicator: c,
            id
        }
    }
}

impl<T: LoRaWANCommunicator> Deref for DebugCommunicator<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.communicator
    }
}
impl<T: LoRaWANCommunicator> DerefMut for DebugCommunicator<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.communicator
    }
}

#[async_trait]
impl<T: LoRaWANCommunicator> LoRaWANCommunicator for DebugCommunicator<T> {
    type Config = T::Config;

    async fn from_config(config: &Self::Config) -> Result<Box<Self>, CommunicatorError> {
        Ok(Box::new(Self {
            communicator: *T::from_config(config).await.unwrap(),
            id: None
        }))
    }

    async fn send_uplink(
        &self,
        bytes: &[u8],
        src: Option<EUI64>,
        dest: Option<EUI64>,
    ) -> Result<(), CommunicatorError> {
        println!(
            "[{:?}] Device {} sending {} to {}",
            SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis(),
            self.id.map(|v| PrettyHexSlice(&*v).to_string())
                .unwrap_or("Unknown".to_owned()),
            PrettyHexSlice(bytes),
            dest.map(|v| PrettyHexSlice(&*v).to_string())
                .unwrap_or("Unknown".to_owned())
        );
        self.communicator.send_uplink(bytes, src, dest).await
    }

    async fn receive_downlink(
        &self,
        timeout: Option<Duration>,
    ) -> Result<HashMap<SpreadingFactor, LoRaPacket>, CommunicatorError> {
        println!(
            "[{:?}] Device {} Waiting for downlink",
            SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis(),
            self.id.map(|v| PrettyHexSlice(&*v).to_string())
                .unwrap_or("Unknown".to_owned())
        );
        let r = self.communicator.receive_downlink(timeout).await?;
        println!(
            "[{:?}] Device {} Ended waiting! Received {} packets: {}",
            SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis(),
            self.id.map(|v| PrettyHexSlice(&*v).to_string())
                .unwrap_or("Unknown".to_owned()), r.values().map(|v| {
                    PrettyHexSlice(&v.payload).to_string()
                }).collect::<Vec<_>>().join(","),
            r.len()
        );
        Ok(r)
    }
}
