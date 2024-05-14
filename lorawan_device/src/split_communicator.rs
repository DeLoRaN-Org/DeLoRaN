use std::time::Duration;

use crate::communicator::{CommunicatorError, LoRaWANCommunicator, ReceivedTransmission};

pub trait LoRaSender {
    type OptionalInfo: Send + Sync;
    fn send(&self, bytes: &[u8], optional_info: Option<Self::OptionalInfo>) -> impl std::future::Future<Output = Result<(), CommunicatorError>> + Send;
}

pub trait LoRaReceiver {
    fn receive(&self, timeout: Option<Duration>) -> impl std::future::Future<Output = Result<Vec<ReceivedTransmission>, CommunicatorError>> + Send;
}

pub trait SplitCommunicator: LoRaWANCommunicator + Send + Sync {
    type Sender: LoRaSender + Send + Sync;
    type Receiver: LoRaReceiver + Send + Sync;

    fn split_communicator(self) -> impl std::future::Future<Output = Result<(Self::Sender, Self::Receiver), CommunicatorError>> + Send;
}