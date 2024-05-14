use std::{
    ops::{Deref, DerefMut},
    time::{Duration, SystemTime},
};

use crate::{
    communicator::{CommunicatorError, LoRaWANCommunicator, ReceivedTransmission}, split_communicator::{LoRaReceiver, LoRaSender, SplitCommunicator}, devices::lorawan_device::LoRaWANDevice
};
use blockchain_api::{exec_bridge::BlockchainExeClient, BlockchainClient};
use lorawan::{
    device::Device,
    utils::{eui::EUI64, PrettyHexSlice},
};

pub struct DebugDevice;
impl DebugDevice {
    pub fn create<T: LoRaWANCommunicator>(
        device: Device,
        communicator: T,
    ) -> LoRaWANDevice<DebugCommunicator<T>> {
        LoRaWANDevice::new(device, DebugCommunicator {
            inner: communicator,
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
            inner: communicator,
            id: None
        })
    }

    pub fn from<T: LoRaWANCommunicator + Send + Sync>(d: LoRaWANDevice<T>) -> LoRaWANDevice<DebugCommunicator<T>> {
        let (device, communicator) = d.into();
        let id = Some(*device.dev_eui());
        LoRaWANDevice::new(device, DebugCommunicator {
            inner: communicator,
            id
        })
    }
}

pub struct DebugCommunicator<T: LoRaWANCommunicator> {
    inner: T,
    id: Option<EUI64>
}

impl <T: LoRaWANCommunicator> DebugCommunicator<T> {
    pub fn set_id(&mut self, id: &EUI64) {
        self.id = Some(*id)
    }

    pub fn from(c: T, id: Option<EUI64>) -> DebugCommunicator<T> where T: LoRaWANCommunicator + Send + Sync {
        DebugCommunicator {
            inner: c,
            id
        }
    }
}

impl<T: LoRaWANCommunicator> Deref for DebugCommunicator<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl<T: LoRaWANCommunicator> DerefMut for DebugCommunicator<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: LoRaWANCommunicator> LoRaWANCommunicator for DebugCommunicator<T> {
    type Config = T::Config;

    async fn from_config(config: &Self::Config) -> Result<Self, CommunicatorError> {
        Ok(Self {
            inner: T::from_config(config).await.unwrap(),
            id: None
        })
    }

    async fn send(
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
        self.inner.send(bytes, src, dest).await
    }

    async fn receive(
        &self,
        timeout: Option<Duration>,
    ) -> Result<Vec<ReceivedTransmission>, CommunicatorError> {
        println!(
            "[{:?}] Device {} Waiting for downlink",
            SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis(),
            self.id.map(|v| PrettyHexSlice(&*v).to_string())
                .unwrap_or("Unknown".to_owned())
        );
        let r = self.inner.receive(timeout).await?;
        println!(
            "[{:?}] Device {} Ended waiting! Received {} packets: {}",
            SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis(),
            self.id.map(|v| PrettyHexSlice(&*v).to_string())
                .unwrap_or("Unknown".to_owned()), r.iter().map(|v| {
                    PrettyHexSlice(&v.transmission.payload).to_string()
                }).collect::<Vec<_>>().join(","),
            r.len()
        );
        Ok(r)
    }
}

pub struct DebugSender<T: LoRaSender> {
    id: Option<EUI64>,
    inner: T
}

impl <T: LoRaSender + Send + Sync> LoRaSender for DebugSender<T> {
    type OptionalInfo = T::OptionalInfo;
    
    fn send(&self, bytes: &[u8], optional_info: Option<T::OptionalInfo>) -> impl std::future::Future<Output = Result<(), crate::communicator::CommunicatorError>> + Send {
        println!(
            "[{:?}] Device {} sending {}",
            SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis(),
            self.id.map(|v| PrettyHexSlice(&*v).to_string())
                .unwrap_or("Unknown".to_owned()),
            PrettyHexSlice(bytes),
        );
        self.inner.send(bytes, optional_info)
    }
    
}

pub struct DebugReceiver<T: LoRaReceiver> {
    id: Option<EUI64>,
    inner: T
}

impl <T: LoRaReceiver + Sync + Send> LoRaReceiver for DebugReceiver<T> {
    async fn receive(
        &self,
        timeout: Option<std::time::Duration>,
    ) -> Result<Vec<crate::communicator::ReceivedTransmission>, crate::communicator::CommunicatorError> {
        println!(
            "[{:?}] Device {} Waiting for downlink",
            SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis(),
            self.id.map(|v| PrettyHexSlice(&*v).to_string())
                .unwrap_or("Unknown".to_owned())
        );
        let r = self.inner.receive(timeout).await?;
        println!(
            "[{:?}] Device {} Ended waiting! Received {} packets: {}",
            SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis(),
            self.id.map(|v| PrettyHexSlice(&*v).to_string())
                .unwrap_or("Unknown".to_owned()), r.iter().map(|v| {
                    PrettyHexSlice(&v.transmission.payload).to_string()
                }).collect::<Vec<_>>().join(","),
            r.len()
        );
        Ok(r)
    }
}

impl <T: SplitCommunicator> SplitCommunicator for DebugCommunicator<T> {
    type Sender = DebugSender<<T as SplitCommunicator>::Sender>;
    type Receiver = DebugReceiver<<T as SplitCommunicator>::Receiver>;

    async fn split_communicator(self) -> Result<(Self::Sender, Self::Receiver), crate::communicator::CommunicatorError> {
        let (sender, receiver) = self.inner.split_communicator().await?;
        Ok((
            DebugSender {
                id: self.id,
                inner: sender
            },
            DebugReceiver {
                id: self.id,
                inner: receiver
            }
        ))
    }
}