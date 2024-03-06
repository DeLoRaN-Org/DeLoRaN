use std::{hash::Hash, time::{Duration, SystemTime, UNIX_EPOCH}};

use pyo3::prelude::*;

use async_trait::async_trait;
use lorawan::{
    physical_parameters::{LoRaBandwidth, CodeRate, SpreadingFactor},
    utils::{errors::LoRaWANError, eui::EUI64},
};
use serde::{Deserialize, Serialize};

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
    UDP(std::io::Error),
    LoRaWANError(LoRaWANError),
}

#[async_trait]
pub trait LoRaWANCommunicator: Send + Sync + Sized {
    type Config: Send + Sync;
    
    async fn from_config(config: &Self::Config) -> Result<Self, CommunicatorError>;
    
    async fn send(
        &self,
        bytes: &[u8],
        src: Option<EUI64>,
        dest: Option<EUI64>,
    ) -> Result<(), CommunicatorError>;

    async fn receive(
        &self,
        timeout: Option<Duration>,
    ) -> Result<Vec<ReceivedTransmission>, CommunicatorError>;
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


#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Position {
    pub fn distance(&self, other: &Position) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2)).sqrt()
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct ArrivalStats {
    pub time: u128,
    pub rssi: f32,
    pub snr: f32,
}

impl Eq for ArrivalStats {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Transmission {
    pub start_position: Position,
    pub start_time: u128,
    pub frequency: f32,
    pub bandwidth: LoRaBandwidth,
    pub spreading_factor: SpreadingFactor,
    pub code_rate: CodeRate,
    pub starting_power: f32,
    pub uplink: bool,

    pub payload: Vec<u8>,
}

impl PartialEq for Transmission {
    fn eq(&self, other: &Self) -> bool {
        self.start_time == other.start_time && self.bandwidth == other.bandwidth && self.spreading_factor == other.spreading_factor && self.code_rate == other.code_rate && self.uplink == other.uplink && self.payload == other.payload
    }
}

impl Eq for Transmission {}

impl Hash for Transmission {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.start_time.hash(state);
        self.bandwidth.hash(state);
        self.spreading_factor.hash(state);
        self.uplink.hash(state);
        self.payload.hash(state);
    }
}

impl Transmission {
    //https://github.com/avbentem/airtime-calculator/blob/master/doc/LoraDesignGuide_STD.pdf
    pub fn time_on_air(&self) -> u128 {
        let header_disabled = 0_u32; // implicit header disabled (H=0) or not (H=1), can only have implicit header with SF6
        let mut data_rate_optimization = 0_u32; // low data rate optimization enabled (=1) or not (=0)
        if self.bandwidth == LoRaBandwidth::BW125 && (self.spreading_factor == SpreadingFactor::SF11 || self.spreading_factor == SpreadingFactor::SF12) {
            data_rate_optimization = 1; // low data rate optimization mandated for BW125 with SF11 and SF12
        }

        let npream = 8_u32; // number of preamble symbol (12.25 from Utz paper)
        let tsym = ((2.0f32).powi(self.spreading_factor.value() as i32) / (self.bandwidth.khz())) * 1000.0;
        let tpream = (npream as f32 + 4.25) * tsym;

        let cr = match self.code_rate {
            CodeRate::CR4_5 => 5,
            CodeRate::CR4_6 => 6,
            CodeRate::CR5_7 => 7,
            CodeRate::CR4_8 => 8,
        } - 4;


        let v1 = ((8 * (self.payload.len()) - 4 * (self.spreading_factor.value() as usize) + 44 - 20 * header_disabled as usize)  //28 + 16 = 44(? -->     payloadSymbNB = 8 + max(math.ceil((8.0*pl-4.0*sf+28+16-20*H)/(4.0*(sf-2*DE)))*(cr+4),0))
            / (4 * ((self.spreading_factor.value() as usize) - 2 * data_rate_optimization as usize))) * (cr + 4);
        let payload_symb_nb = 8 + (if v1 > 0 { v1 } else { 0 });
        let tpayload = (payload_symb_nb as f32) * tsym;
        (tpream + tpayload).round() as u128
    }

    pub fn ended(&self) -> bool {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() > self.start_time + self.time_on_air()
    }
}


#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReceivedTransmission {
    pub transmission: Transmission,
    pub arrival_stats: ArrivalStats,
}

impl Hash for ReceivedTransmission {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.transmission.hash(state);
    }
}

impl ReceivedTransmission {
    pub fn time_on_air(&self) -> u128 {
        self.transmission.time_on_air()
    }
}


impl From<LoRaPacket> for ReceivedTransmission {
    fn from(packet: LoRaPacket) -> Self {
        Self {
            transmission: Transmission {
                start_position: Position { x: 0.0, y: 0.0, z: 0.0 },
                start_time: 0,
                frequency: 868_000_000.0,
                bandwidth: packet.bw.into(),
                spreading_factor: SpreadingFactor::new(packet.sf),
                code_rate: CodeRate::CR4_5,
                starting_power: packet.rssi,
                uplink: false,
                payload: packet.payload,
            },
            arrival_stats: ArrivalStats {
                time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
                rssi: packet.rssi,
                snr: packet.snr,
            },
        }
    }
}