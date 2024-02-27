use std::{ops::Deref, convert::{TryFrom, TryInto}, fmt::Display};

use hex::{ToHex, FromHex};
use serde::{Serialize, Deserialize};

use crate::utils::{errors::LoRaWANError, PrettyHexSlice};

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Key([u8; 16]);

impl Key {
    pub fn get_raw_key(&self) -> &[u8] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        self.0.encode_hex::<String>()
    }

    pub fn from_hex(hex_str: &str) -> Result<Self,LoRaWANError> {
        let mut v: [u8; 16] = [0; 16];
        let vec = Vec::from_hex(hex_str).map_err(|_| LoRaWANError::InvalidKeyBuffer)?;
        if vec.len() != 16 {
            return Err(LoRaWANError::InvalidKeyBuffer);
        }
        v.copy_from_slice(&vec); 
        Ok(Key::from(v))
    } 
}

impl From<[u8; 16]> for Key {
    fn from(b: [u8; 16]) -> Self {
        Self(b)
    }
}

impl TryFrom<Vec<u8>> for Key {
    type Error=LoRaWANError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        //println!("{value:?}; {}", value.len());
        if value.len() < 16 {
            Err(LoRaWANError::InvalidKeyBuffer)
        }
        else {
            value[0..16].try_into().map(Self).map_err(|_|LoRaWANError::InvalidKeyBuffer)
        }
    }
}

impl Deref for Key {
    type Target = [u8;16];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", PrettyHexSlice(&self.0))
    }
}