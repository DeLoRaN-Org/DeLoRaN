use crate::device::Device;

use super::errors::LoRaWANError;


//TODO SOSTITUIRE VEC<U8> CON BOX<[U8]>
pub trait ToBytes {
    fn to_bytes(&self) -> Vec<u8>;
}

//TODO SOSTITUIRE VEC<U8> CON BOX<[U8]>
pub trait ToBytesWithContext {
    fn to_bytes_with_context(&self, device_context: &Device) -> Result<Vec<u8>, LoRaWANError>;
}