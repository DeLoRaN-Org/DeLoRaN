use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::regional_parameters::region::Region;

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Default)]
pub enum LoRaBandwidth {
    #[default]
    BW125,
    BW250,
    BW500,
}

impl From<f32> for LoRaBandwidth {
    fn from(bw: f32) -> Self {
        let u32_bw = bw as u32;

        if u32_bw < 1000 {
            match u32_bw {
                125 => LoRaBandwidth::BW125,
                250 => LoRaBandwidth::BW250,
                500 => LoRaBandwidth::BW500,
                _ => LoRaBandwidth::BW125,
            }
        } else {
            match u32_bw {
                125_000 => LoRaBandwidth::BW125,
                250_000 => LoRaBandwidth::BW250,
                500_000 => LoRaBandwidth::BW500,
                _ => LoRaBandwidth::BW125,
            }
        }
    }
}

impl LoRaBandwidth {
    pub fn hz(&self) -> f32 {
        match self {
            LoRaBandwidth::BW125 => 125_000.0,
            LoRaBandwidth::BW250 => 250_000.0,
            LoRaBandwidth::BW500 => 500_000.0,
        }
    }
    
    pub fn khz(&self) -> f32 {
        match self {
            LoRaBandwidth::BW125 => 125.0,
            LoRaBandwidth::BW250 => 250.0,
            LoRaBandwidth::BW500 => 500.0,
        }
    }
}



#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Default)]
pub enum SpreadingFactor {
    #[default]
    SF7,
    SF8,
    SF9,
    SF10,
    SF11,
    SF12,
}

impl SpreadingFactor {    
    pub fn new(sf: u8) -> Self {
        match sf {
            0..=6 => SpreadingFactor::SF7,
            7 => SpreadingFactor::SF7,
            8 => SpreadingFactor::SF8,
            9 => SpreadingFactor::SF9,
            10 => SpreadingFactor::SF10,
            11 => SpreadingFactor::SF11,
            12 => SpreadingFactor::SF12,
            _ => SpreadingFactor::SF12,
        }
    }


    pub fn value(&self) -> u8 {
        match self {
            SpreadingFactor::SF7 => 7,
            SpreadingFactor::SF8 => 8,
            SpreadingFactor::SF9 => 9,
            SpreadingFactor::SF10 => 10,
            SpreadingFactor::SF11 => 11,
            SpreadingFactor::SF12 => 12,
        }
    }
}

impl Display for SpreadingFactor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SF{}", self.value())
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum DataRate {
    DR0,   //SF 12 BW 125
    DR1,   //SF 11 BW 125
    DR2,   //SF 10 BW 125
    DR3,   //SF 9 BW 125
    DR4,   //SF 8 BW 125
    DR5,   //SF 7 BW 125
    DR6,   //SF 6 BW 125
    DR7,   //FSK 50kbps
    DR8,
    DR9,
    DR10,
    DR11,
    DR12,
    DR13,
    DR14,
    DR15,
}

impl DataRate {
    pub fn new(i: u8) -> Self {
        match i {
            0 => DataRate::DR0,
            1 => DataRate::DR1,
            2 => DataRate::DR2,
            3 => DataRate::DR3,
            4 => DataRate::DR4,
            5 => DataRate::DR5,
            6 => DataRate::DR6,
            7 => DataRate::DR7,
            8 => DataRate::DR8,
            9 => DataRate::DR9,
            10 => DataRate::DR10,
            11 => DataRate::DR11,
            12 => DataRate::DR12,
            13 => DataRate::DR13,
            14 => DataRate::DR14,
            15 => DataRate::DR15,
            16.. => DataRate::DR15,
        }
    }

    pub fn value(&self) -> u8 {
        match self {
            DataRate::DR0 => 0,
            DataRate::DR1 => 1,
            DataRate::DR2 => 2,
            DataRate::DR3 => 3,
            DataRate::DR4 => 4,
            DataRate::DR5 => 5,
            DataRate::DR6 => 6,
            DataRate::DR7 => 7,
            DataRate::DR8 => 8,
            DataRate::DR9 => 9,
            DataRate::DR10 => 10,
            DataRate::DR11 => 11,
            DataRate::DR12 => 12,
            DataRate::DR13 => 13,
            DataRate::DR14 => 14,
            DataRate::DR15 => 15,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CodeRate {
    #[default]
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
