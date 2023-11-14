use std::{collections::HashMap, time::Duration};

use pyo3::prelude::*;

use async_trait::async_trait;
use lorawan::{
    physical_parameters::SpreadingFactor,
    utils::{errors::LoRaWANError, eui::EUI64},
};

pub fn extract_dev_id(dev_eui: Option<EUI64>) -> u16 {
    dev_eui.map_or(0, |v| {
        let prime: u64 = 31;
        let mut hash: u64 = 0;
        for &value in (*v).iter() {
            hash = hash.wrapping_mul(prime);
            hash = hash.wrapping_add(u64::from(value));
        }
        let hash_bytes = hash.to_ne_bytes();
        let mut folded_hash: u16 = 0;
        for value in hash_bytes.chunks(2) {
            let combined: u16 = ((value[0] as u16) << 8) | (value[1] as u16);
            folded_hash ^= combined;
        }
        if folded_hash == 0 {
            folded_hash.wrapping_add(1)
        } else {
            folded_hash
        }
    })
}

#[derive(Debug)]
pub enum CommunicatorError {
    Radio(String),
    TCP(std::io::Error),
    LoRaWANError(LoRaWANError),
}

#[async_trait]
pub trait LoRaWANCommunicator: Send + Sync {
    type Config: Send + Sync;
    
    async fn from_config(config: &Self::Config) -> Result<Box<Self>, CommunicatorError>;
    
    async fn send_uplink(
        &self,
        bytes: &[u8],
        src: Option<EUI64>,
        dest: Option<EUI64>,
    ) -> Result<(), CommunicatorError>;

    async fn receive_downlink(
        &self,
        timeout: Option<Duration>,
    ) -> Result<HashMap<SpreadingFactor, LoRaPacket>, CommunicatorError>;
}

impl From<LoRaWANError> for CommunicatorError {
    fn from(value: LoRaWANError) -> Self {
        CommunicatorError::LoRaWANError(value)
    }
}

impl From<std::io::Error> for CommunicatorError {
    fn from(value: std::io::Error) -> Self {
        CommunicatorError::TCP(value)
    }
}

#[derive(Debug, Default, Clone)]
pub struct LoRaPacket {
    pub payload: Vec<u8>,
    pub src: u16,
    pub dst: u16,
    pub seqn: u8,
    pub hdr_ok: u8,
    pub has_crc: u8,
    pub crc_ok: u8,
    pub cr: u8,
    pub ih: u8,
    pub sf: u8,
    pub bw: f32,
    pub rssi: f32,
    pub snr: f32,
}

impl<'source> FromPyObject<'source> for LoRaPacket {
    fn extract(ob: &'source PyAny) -> PyResult<Self> {
        Ok(Self {
            payload: ob.getattr("payload")?.extract()?,
            src: ob.getattr("src")?.extract()?,
            dst: ob.getattr("dst")?.extract()?,
            seqn: ob.getattr("seqn")?.extract()?,
            hdr_ok: ob.getattr("hdr_ok")?.extract()?,
            has_crc: ob.getattr("has_crc")?.extract()?,
            crc_ok: ob.getattr("crc_ok")?.extract()?,
            cr: ob.getattr("cr")?.extract()?,
            ih: ob.getattr("ih")?.extract()?,
            sf: ob.getattr("SF")?.extract()?,
            bw: ob.getattr("BW")?.extract()?,
            rssi: ob.getattr("rssi")?.extract()?,
            snr: ob.getattr("snr")?.extract()?,
        })
    }
}
