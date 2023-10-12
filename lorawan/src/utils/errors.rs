use std::array::TryFromSliceError;

use openssl::error::ErrorStack;

#[derive(Debug)]
pub enum LoRaWANError {
    SessionContextMissing,
    ProprietaryContextMissing,
    FPortInvalidValue,
    FCtrlNotCoherentWithPayload,
    MHDRNotCoherentWithPayload,
    MHDRNotCoherentWithContext,
    InvalidKeyBuffer,
    ContextNeeded,
    PacketContextNeeded,
    InvalidEUI64Buffer,
    OpenSSLErrorStack(ErrorStack),
    MalformedMACCommand,
    
    InvalidMic,
    InvalidBufferLength,
    InvalidBufferContent,
    MissingDownlink,
}

impl From<ErrorStack> for LoRaWANError {
    fn from(e: ErrorStack) -> Self {
        eprintln!("{e}");
        LoRaWANError::OpenSSLErrorStack(e)
    }
}

impl From<TryFromSliceError> for LoRaWANError {
    fn from(e: TryFromSliceError) -> Self {
        eprintln!("{e}");
        LoRaWANError::InvalidBufferLength
    }
}