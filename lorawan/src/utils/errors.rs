use std::{array::TryFromSliceError, error::Error, fmt::Display};

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
    InvalidNonce,
    InvalidBufferLength,
    InvalidBufferContent,
    InvalidDevAddr,
    MissingDownlink,
}

impl From<ErrorStack> for LoRaWANError {
    fn from(e: ErrorStack) -> Self {
        //eprintln!("{e}");
        LoRaWANError::OpenSSLErrorStack(e)
    }
}

impl From<TryFromSliceError> for LoRaWANError {
    fn from(_e: TryFromSliceError) -> Self {
        //eprintln!("{e}");
        LoRaWANError::InvalidBufferLength
    }
}

impl Display for LoRaWANError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoRaWANError::SessionContextMissing => write!(f, "Session context missing"),
            LoRaWANError::ProprietaryContextMissing => write!(f, "Proprietary context missing"),
            LoRaWANError::FPortInvalidValue => write!(f, "FPort invalid value"),
            LoRaWANError::FCtrlNotCoherentWithPayload => write!(f, "FCtrl not coherent with payload"),
            LoRaWANError::MHDRNotCoherentWithPayload => write!(f, "MHDR not coherent with payload"),
            LoRaWANError::MHDRNotCoherentWithContext => write!(f, "MHDR not coherent with context"),
            LoRaWANError::InvalidKeyBuffer => write!(f, "Invalid key buffer"),
            LoRaWANError::ContextNeeded => write!(f, "Context needed"),
            LoRaWANError::PacketContextNeeded => write!(f, "Packet context needed"),
            LoRaWANError::InvalidEUI64Buffer => write!(f, "Invalid EUI64 buffer"),
            LoRaWANError::OpenSSLErrorStack(e) => write!(f, "OpenSSL error: {}", e),
            LoRaWANError::MalformedMACCommand => write!(f, "Malformed MAC command"),
            LoRaWANError::InvalidMic => write!(f, "Invalid MIC"),
            LoRaWANError::InvalidNonce => write!(f, "Invalid nonce"),
            LoRaWANError::InvalidBufferLength => write!(f, "Invalid buffer length"),
            LoRaWANError::InvalidBufferContent => write!(f, "Invalid buffer content"),
            LoRaWANError::InvalidDevAddr => write!(f, "Invalid DevAddr"),
            LoRaWANError::MissingDownlink => write!(f, "Missing downlink"),
        }
    }
}

impl Error for LoRaWANError {}