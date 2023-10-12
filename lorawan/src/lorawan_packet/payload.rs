use super::{
    join::{JoinAcceptPayload, JoinRequestPayload, RejoinRequestPayload},
    mac_payload::MACPayload,
};
use crate::{
    device::Device,
    utils::{errors::LoRaWANError, traits::{ToBytes, ToBytesWithContext}},
};
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum Payload {
    JoinRequest(JoinRequestPayload),
    JoinAccept(JoinAcceptPayload),
    RejoinRequest(RejoinRequestPayload),
    MACPayload(MACPayload),
    Proprietary(Vec<u8>),
}

impl Default for Payload {
    fn default() -> Payload {
        Payload::MACPayload(MACPayload::default())
    }
}

impl ToBytesWithContext for Payload {
    fn to_bytes_with_context(&self, device_context: &Device) -> Result<Vec<u8>, LoRaWANError> {
        match self {
            Payload::JoinRequest(jr) => Ok(jr.to_bytes()),
            Payload::JoinAccept(ja) => Ok(ja.to_bytes()),
            Payload::RejoinRequest(rr) => Ok(rr.to_bytes()),
            Payload::MACPayload(p) => p.to_bytes_with_context(device_context),
            Payload::Proprietary(buffer) => {  
                //device_context.proprietary_payload_handlers()
                //.ok_or(LoRaWANError::ProprietaryContextMissing)?
                //.custom_to_bytes_with_context()(buffer.clone(), device_context)
                Ok(buffer.clone())
            }
        }
    }
}
