use lorawan::utils::errors::LoRaWANError;
use tokio::sync::oneshot::error::RecvError;


#[derive(Debug)]
pub enum ASError {
    CommandTransmissionFailed(String),


    LoRaWANError(LoRaWANError)
}


impl From<RecvError> for ASError {
    fn from(e: RecvError) -> Self {
        Self::CommandTransmissionFailed(e.to_string())
    }
}

impl From<LoRaWANError> for ASError {
    fn from(e: LoRaWANError) -> Self {
        Self::LoRaWANError(e)
    }
}