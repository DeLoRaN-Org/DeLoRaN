use std::{hash::Hash, time::{Duration, SystemTime, UNIX_EPOCH}};

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

pub trait LoRaWANCommunicator: Send + Sync + Sized {
    type Config: Send + Sync;
    
    fn from_config(config: &Self::Config) -> impl std::future::Future<Output = Result<Self, CommunicatorError>> + Send;
    
    fn send(
        &self,
        bytes: &[u8],
        src: Option<EUI64>,
        dest: Option<EUI64>,
    ) -> impl std::future::Future<Output = Result<(), CommunicatorError>> + Send;

    fn receive(
        &self,
        timeout: Option<Duration>,
    ) -> impl std::future::Future<Output = Result<Vec<ReceivedTransmission>, CommunicatorError>> + Send;
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
        self.start_time == other.start_time && 
        self.bandwidth == other.bandwidth && 
        self.spreading_factor == other.spreading_factor && 
        self.code_rate == other.code_rate && 
        self.uplink == other.uplink && 
        self.payload == other.payload
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

impl PartialOrd for Transmission {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.start_time.cmp(&other.start_time))
    }
}

impl Ord for Transmission {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.start_time.cmp(&other.start_time)
    }
}


impl Transmission {
    //https://github.com/avbentem/airtime-calculator/blob/master/doc/LoraDesignGuide_STD.pdf
    pub fn time_on_air(&self) -> u128 {
        let header_disabled = 0_usize; // implicit header disabled (H=0) or not (H=1), can only have implicit header with SF6
        let mut data_rate_optimization = 0_u32; // low data rate optimization enabled (=1) or not (=0)
        if self.bandwidth == LoRaBandwidth::BW125 && (self.spreading_factor == SpreadingFactor::SF11 || self.spreading_factor == SpreadingFactor::SF12) {
            data_rate_optimization = 1; // low data rate optimization mandated for BW125 with SF11 and SF12
        }

        let npream = 8_u32; // number of preamble symbol (12.25 from Utz paper)
        let tsym = (2.0f32).powi(self.spreading_factor.value() as i32) / (self.bandwidth.khz());
        let tpream = (npream as f32 + 4.25) * tsym;

        let cr = match self.code_rate {
            CodeRate::CR4_5 => 1,
            CodeRate::CR4_6 => 2,
            CodeRate::CR5_7 => 3,
            CodeRate::CR4_8 => 4,
        };

        let num = (8* self.payload.len() - 4 * (self.spreading_factor.value() as usize) + 28 + 16 - 20 * header_disabled) as f32;
        let den = 4.0 * (self.spreading_factor.value() as f32 - 2.0 * data_rate_optimization as f32);
        let v1 = (num / den).ceil() * (cr as f32 + 4.0);

        let payload_symb_nb = 8.0 + (if v1 > 0.0 { v1 } else { 0.0 });
        let tpayload = payload_symb_nb * tsym;
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