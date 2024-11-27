use serde::{Deserialize, Serialize};

use crate::utils::traits::ToBytes;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Major {
    R1,  //0
    RFU, // 1
}

impl ToBytes for Major {
    fn to_bytes(&self) -> Vec<u8> {
        let v = match self {
            Major::R1 => 0b00000000,
            Major::RFU => 0b00000001,
        };
        vec![v]
    }
}

impl Default for Major {
    fn default() -> Self {
        Self::R1
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MType {
    JoinRequest,
    JoinAccept,
    UnconfirmedDataUp,
    UnconfirmedDataDown,
    ConfirmedDataUp,
    ConfirmedDataDown,
    RejoinRequest,
    Proprietary,
}

impl From<u8> for MType {
    fn from(byte: u8) -> Self {
        match byte {
            0b00000000 => MType::JoinRequest,
            0b00100000 => MType::JoinAccept,
            0b01000000 => MType::UnconfirmedDataUp,
            0b01100000 => MType::UnconfirmedDataDown,
            0b10000000 => MType::ConfirmedDataUp,
            0b10100000 => MType::ConfirmedDataDown,
            0b11000000 => MType::RejoinRequest,
            _          => MType::Proprietary,
        }
    }
}

impl ToBytes for MType {
    fn to_bytes(&self) -> Vec<u8> {
        let v = match self {
            MType::JoinRequest => 0b00000000,
            MType::JoinAccept => 0b00100000,
            MType::UnconfirmedDataUp => 0b01000000,
            MType::UnconfirmedDataDown => 0b01100000,
            MType::ConfirmedDataUp => 0b10000000,
            MType::ConfirmedDataDown => 0b10100000,
            MType::RejoinRequest => 0b11000000,
            MType::Proprietary => 0b11100000,
        };
        vec![v]
    }
}


impl Default for MType {
    fn default() -> Self {
        Self::UnconfirmedDataDown
    }
}


#[derive(Default, Debug, Clone, Serialize, Deserialize)]
///1 byte :: 3 bits for mtype | 3 bits for rfu | 2 bits for major
pub struct MHDR {
    mtype: MType,
    rfu: u8,
    major: Major,
}


impl MHDR {
    pub fn new(mtype: MType, major: Major) -> Self {
        Self {
            mtype,
            rfu: 0,
            major
        }
    }

    pub fn mtype(&self) -> MType {
        self.mtype
    }
    
    pub fn major(&self) -> Major {
        self.major
    }

    pub fn from_bytes(mhdr_byte: u8) -> Self {
        let mtype_bits = (mhdr_byte & 0b11100000) >> 5;
        let major_bits = mhdr_byte & 0b00000011;
        
        let mtype = match mtype_bits {
            0b000 => MType::JoinRequest,
            0b001 => MType::JoinAccept,
            0b010 => MType::UnconfirmedDataUp,
            0b011 => MType::UnconfirmedDataDown,
            0b100 => MType::ConfirmedDataUp,
            0b101 => MType::ConfirmedDataDown,
            0b110 => MType::RejoinRequest,
            _     => MType::Proprietary,
        };

        let major = match major_bits {  
            0b00 => Major::R1,
            _    => Major::RFU
        };

        MHDR::new(mtype, major)
    }

    pub fn is_join_rejoin(&self) -> bool {
        self.mtype == MType::JoinRequest || self.mtype == MType::RejoinRequest
    }
}

impl ToBytes for MHDR {
    fn to_bytes(&self) -> Vec<u8> {
        let mut v = self.mtype.to_bytes();
        v[0] = v[0] | self.rfu | self.major.to_bytes()[0];
        v
    }
}