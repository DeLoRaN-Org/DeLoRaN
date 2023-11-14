use std::io::Error;

use blockchain_api::BlockchainError;
use lorawan::utils::errors::LoRaWANError;
use tokio::sync::oneshot::error::RecvError;

#[derive(Debug)]
pub enum NCError {
    ConfigurationMissing,
    CommandTransmissionFailed(String),
    DBError,
    IOError(Error),

    InvalidJoinRequest(String),
    InvalidUplink(String),
    InvalidDownlink(String),
    UnknownDevEUI([u8; 8]),
    UnknownDevAddr([u8; 4]),
    LoRaWANError(LoRaWANError),

    BlockchainError(BlockchainError)
}

impl From<BlockchainError> for NCError {
    fn from(e: BlockchainError) -> Self {
        Self::BlockchainError(e)
    }
}

impl From<RecvError> for NCError {
    fn from(e: RecvError) -> Self {
        Self::CommandTransmissionFailed(e.to_string())
    }
}

impl From<LoRaWANError> for NCError {
    fn from(e: LoRaWANError) -> Self {
        Self::LoRaWANError(e)
    }
}