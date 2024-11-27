use std::{convert::TryFrom, fmt::Display, ops::Deref};

use hex::FromHex;
use serde::{Serialize, Deserialize};

use crate::utils::PrettyHexSlice;

use super::errors::LoRaWANError;


#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EUI64([u8; 8]);

impl EUI64 {
    pub fn from_hex(hex_str: &str) -> Result<Self, LoRaWANError> {
        let mut v: [u8; 8] = [0; 8];
        let vec = Vec::from_hex(hex_str).map_err(|_| LoRaWANError::InvalidKeyBuffer)?;
        if vec.len() != 8 {
            return Err(LoRaWANError::InvalidKeyBuffer);
        }
        v.copy_from_slice(&vec); 
        Ok(EUI64::from(v))
    }
}

impl TryFrom<&str> for EUI64 {
    type Error = LoRaWANError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let mut bytes: [u8; 8] = [0; 8];
        hex::decode_to_slice(s, &mut bytes)
            .map(|_| Self ( bytes ))
            .map_err(|_| LoRaWANError::InvalidEUI64Buffer)
    }
}

impl From<&EUI64> for String {
    fn from(value: &EUI64) -> Self {
        PrettyHexSlice(&**value).to_string()
    }
}

impl From<[u8; 8]> for EUI64 {
    fn from(s: [u8; 8]) -> Self {
        Self(s)
    }
}

impl Deref for EUI64 {
    type Target=[u8;8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for EUI64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", PrettyHexSlice(&self.0))
    }
}
