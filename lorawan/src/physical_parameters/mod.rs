use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::regional_parameters::region::Region;

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]

pub struct SpreadingFactor(u8);
impl SpreadingFactor {
    pub fn new(i: u8) -> Self {
        match i {
            0..=6 => Self(7),
            7..=12 => Self(i),
            13.. => Self(12),
        }
    }
    
    pub fn value(&self) -> u8 {
        match self.0 {
            0..=6 => 7,
            7..=12 => self.0,
            13.. => 12,
        }
    }
}

impl Display for SpreadingFactor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SF{}", self.value())
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct DataRate(u8);

impl DataRate {
    pub fn new(i: u8) -> Self {
        match i {
            0..=15 => Self(i),
            16.. => Self(15),
        }
    }

    pub fn value(&self) -> u8 {
        match self.0 {
            0..=15 => self.0,
            16.. => 15,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodeRate {
    CR4_5,
    CR4_6,
    CR5_7,
    CR4_8,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadioParameters {
    pub region: Region,
    pub spreading_factor: SpreadingFactor,
    pub data_rate: DataRate,
    pub rx_gain: u8,
    pub tx_gain: u8,
    pub bandwidth: u32,
    pub rx_freq: u32,
    pub tx_freq: u32,
    pub sample_rate: u32,
    pub rx_chan_id: u8,
    pub tx_chan_id: u8,
}
