use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::regional_parameters::region::Region;

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]


pub enum SpreadingFactor {
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
    DR0,
    DR1,
    DR2,
    DR3,
    DR4,
    DR5,
    DR6,
    DR7,
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
