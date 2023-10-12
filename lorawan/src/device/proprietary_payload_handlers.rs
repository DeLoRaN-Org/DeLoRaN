use std::fmt::Debug;

use crate::utils::errors::LoRaWANError;
use super::Device;

type ToByteFunction = fn(Vec<u8>) -> Result<Vec<u8>, LoRaWANError>;
type ToByteWithContextFunction = fn(Vec<u8>, &Device) -> Result<Vec<u8>, LoRaWANError>;
type MicFunction = fn(&[u8]) -> Result<[u8; 4], LoRaWANError>;

pub struct ProprietaryPayloadHandlers {
    custom_to_bytes: ToByteFunction,
    custom_to_bytes_with_context: ToByteWithContextFunction,
    custom_mic_function: MicFunction,
}

impl Debug for ProprietaryPayloadHandlers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProprietaryPayloadHandlers").field("present", &true).finish()
    }
}

impl ProprietaryPayloadHandlers {
    pub fn new(
        custom_to_bytes: ToByteFunction,
        custom_to_bytes_with_context: ToByteWithContextFunction,
        custom_mic_function: MicFunction,
    ) -> Self {
        Self {
            custom_mic_function,
            custom_to_bytes,
            custom_to_bytes_with_context,
        }
    }

    /// Get the proprietary payload's custom to_bytes function.
    pub fn custom_to_bytes(&self) -> ToByteFunction {
        self.custom_to_bytes
    }

    /// Get the proprietary payload's custom to_bytes_with_context function.
    pub fn custom_to_bytes_with_context(&self) -> ToByteWithContextFunction {
        self.custom_to_bytes_with_context
    }

    /// Get the proprietary payload's custom mic function.
    pub fn custom_mic_function(&self) -> MicFunction {
        self.custom_mic_function
    }
}

impl Default for ProprietaryPayloadHandlers {
    fn default() -> Self {
        Self { 
            custom_to_bytes: |vect| {Ok(vect)}, 
            custom_to_bytes_with_context: |vect, _| {Ok(vect)}, 
            custom_mic_function: |_| {Ok([0,0,0,0])} 
        }
    }
}