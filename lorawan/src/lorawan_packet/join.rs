use std::convert::TryInto;

use serde::{Serialize, Deserialize};

use crate::utils::{errors::LoRaWANError, eui::EUI64, traits::ToBytes};



#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub enum JoinRequestType {
    JoinRequest,
    RejoinRequest0,
    RejoinRequest1,
    RejoinRequest2,
}

impl Default for JoinRequestType {
    fn default() -> Self {
        Self::JoinRequest
    }
}

impl JoinRequestType {
    pub fn to_byte(&self) -> u8 {
        match self {
            JoinRequestType::JoinRequest => 0xff,
            JoinRequestType::RejoinRequest0 => 0x00,
            JoinRequestType::RejoinRequest1 => 0x01,
            JoinRequestType::RejoinRequest2 => 0x02,
        }
    }
    
    pub fn is_rejoin(&self) -> bool {
        match self {
            JoinRequestType::RejoinRequest0 |
            JoinRequestType::RejoinRequest1 |
            JoinRequestType::RejoinRequest2 => true,
            JoinRequestType::JoinRequest => false,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct JoinAcceptPayload {
    join_req_type: JoinRequestType,

    join_nonce: [u8; 3],
    home_net_id: [u8; 3],
    dev_addr: [u8; 4],
    ///7: OptNeg, 6..4 RX1DRoffset 3..0 RX2DataRate //TODO forse fare una struct?
    dl_settings: u8,
    rx_delay: u8,
    ///TODO region specific and optional
    cf_list: Option<[u8; 16]>,
}


#[allow(clippy::too_many_arguments)]
impl JoinAcceptPayload {
    pub fn new(
        join_req_type: JoinRequestType,
        join_nonce: [u8; 3],
        home_net_id: [u8; 3],
        dev_addr: [u8; 4],
        dl_settings: u8,
        rx_delay: u8,
        cf_list: Option<[u8; 16]>,
    ) -> Self {
        Self {
            join_req_type,
            join_nonce,
            home_net_id,
            dev_addr,
            dl_settings,
            rx_delay,
            cf_list,
        }
    }
    /// Get the join accept's join nonce.
    pub fn join_nonce(&self) -> &[u8; 3] {
        &self.join_nonce
    }

    /// Get the join accept's home net id.
    pub fn home_net_id(&self) -> &[u8; 3] {
        &self.home_net_id
    }

    /// Get the join accept's dev addr.
    pub fn dev_addr(&self) -> &[u8; 4] {
        &self.dev_addr
    }

    /// Get the join accept's dl settings.
    pub fn dl_settings(&self) -> u8 {
        self.dl_settings
    }

    pub fn opt_neg(&self) -> bool {
        (self.dl_settings & 0b10000000) > 0
    }

    pub fn rx1_dr_offset(&self) -> u8 {
        (self.dl_settings & 0b01110000) >> 4
    }

    pub fn rx2_data_rate(&self) -> u8 {
        self.dl_settings & 0b00001111
    }

    /// Get the join accept's rx delay.
    pub fn rx_delay(&self) -> u8 {
        self.rx_delay
    }

    /// Get the join accept's cf list.
    pub fn cf_list(&self) -> &Option<[u8; 16]>  {
        &self.cf_list
    }

    pub fn is_rejoin(&self) -> bool {
        self.join_req_type != JoinRequestType::JoinRequest
    }

    /// Get the join accept's rejoin id.
    pub fn join_req_type(&self) -> &JoinRequestType {
        &self.join_req_type
    }
    pub fn from_bytes(bytes: &[u8], join_req_type: &JoinRequestType) -> Result<Self, LoRaWANError> {
        //let (bytes, mic) = bytes.split_at(len - 4);
        let len = bytes.len();
        if len != 12 && len != 28 {
            Err(LoRaWANError::InvalidBufferLength)
        }
        else {
            let mut join_nonce: [u8;3] = bytes[0..3].try_into()?;
            join_nonce.reverse(); 
            let mut home_net_id: [u8;3] = bytes[3..6].try_into()?;
            home_net_id.reverse();
            let mut dev_addr: [u8;4] = bytes[6..10].try_into()?;
            dev_addr.reverse();
    
            let dl_settings = bytes[10];
            let rx_delay = bytes[11];
            let cf_list = if len > 12 {
                let mut list = [0; 16];
                list.copy_from_slice(&bytes[12..]);
                Some(list)
            } else {
                None    
            };
            Ok(Self {
                join_req_type: *join_req_type,
                join_nonce,
                home_net_id,
                dev_addr,
                dl_settings,
                rx_delay,
                cf_list,
            })
        }
    }
}

impl ToBytes for JoinAcceptPayload {
    fn to_bytes(&self) -> Vec<u8> {
        let mut ret = Vec::new();

        let mut join_nonce_reversed = self.join_nonce;
        join_nonce_reversed.reverse(); 
        let mut home_net_id_reversed = self.home_net_id;
        home_net_id_reversed.reverse();
        let mut dev_addr_reversed = self.dev_addr;
        dev_addr_reversed.reverse();


        ret.extend_from_slice(&join_nonce_reversed);
        ret.extend_from_slice(&home_net_id_reversed);
        ret.extend_from_slice(&dev_addr_reversed);
        ret.push(self.dl_settings);
        ret.push(self.rx_delay);
        if let Some(cflist) = &self.cf_list {
            ret.extend_from_slice(cflist);
        }
        ret 
    }
}

#[derive(Default, Debug, Clone)]
pub struct JoinRequestPayload {
    join_eui: EUI64,
    dev_eui: EUI64,
    dev_nonce: u16,
}

impl JoinRequestPayload {
    pub fn new(join_eui: EUI64, dev_eui: EUI64, dev_nonce: u16) -> Self {
        Self {
            join_eui,
            dev_eui,
            dev_nonce,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, LoRaWANError> {
        if bytes.len() != 18 {
            Err(LoRaWANError::InvalidBufferLength)
        }
        else {
            let mut join_eui: [u8; 8] = bytes[0..8].try_into()?;
            join_eui.reverse(); 
            let mut dev_eui: [u8; 8] = bytes[8..16].try_into()?;
            dev_eui.reverse();
            let dev_nonce = u16::from_le_bytes(bytes[16..18].try_into()?);

            Ok(Self {
                join_eui: EUI64::from(join_eui),
                dev_eui: EUI64::from(dev_eui),
                dev_nonce,
            })
        }
    }

    pub fn join_eui(&self) -> &EUI64 {
        &self.join_eui
    }
    pub fn dev_eui(&self) -> &EUI64 {
        &self.dev_eui
    }
    pub fn dev_nonce(&self) -> u16 {
        self.dev_nonce
    }
}

impl ToBytes for JoinRequestPayload {
    fn to_bytes(&self) -> Vec<u8> {
        let mut ret = Vec::new();

        let mut join_eui_reversed = *self.join_eui;
        join_eui_reversed.reverse(); 
        let mut dev_eui_reversed = *self.dev_eui;
        dev_eui_reversed.reverse();

        ret.extend_from_slice(&join_eui_reversed);
        ret.extend_from_slice(&dev_eui_reversed);
        ret.extend_from_slice(&self.dev_nonce.to_le_bytes());
        ret
    }
}

#[derive(Debug, Clone)]
pub enum RejoinRequestPayload {
    T1(ReJoinRequest1),
    T02(ReJoinRequest02),
}

impl RejoinRequestPayload {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, LoRaWANError> {
        let len = bytes.len();
        if len != 14 && len != 19  {
            Err(LoRaWANError::InvalidBufferLength)
        }
        else {
            match bytes[0] {
                0 | 2 => {
                    Ok(RejoinRequestPayload::T02(ReJoinRequest02::from_bytes(bytes)?))
                },
                1 => {
                    Ok(RejoinRequestPayload::T1(ReJoinRequest1::from_bytes(bytes)?))

                },
                _ => {Err(LoRaWANError::InvalidBufferContent)}
            }
        }
    }
}

impl ToBytes for RejoinRequestPayload {
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            RejoinRequestPayload::T1(r) => r.to_bytes(),
            RejoinRequestPayload::T02(r) => r.to_bytes(),
        }
    }
}

#[derive(Copy, Clone, Default, Debug)]
pub struct ReJoinRequest02 {
    is_type_zero: bool,

    net_id: [u8; 3],
    dev_eui: EUI64,
    rj_count0: u16,
}

impl ReJoinRequest02 {
    pub fn new(is_type_zero: bool, net_id: [u8; 3], dev_eui: EUI64, rj_count0: u16) -> Self {
        Self {
            is_type_zero,
            net_id,
            dev_eui,
            rj_count0,
        }
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, LoRaWANError> {
        if bytes.len() != 14 {
            Err(LoRaWANError::InvalidBufferLength)
        }
        else {
            let eui_bytes: [u8; 8] = bytes[4..12].try_into()?;
            Ok(Self {
                is_type_zero: bytes[0] == 0,
                net_id: bytes[1..4].try_into()?,
                dev_eui: EUI64::from(eui_bytes),
                rj_count0: u16::from_le_bytes(bytes[12..14].try_into()?),
                
            })
        }
    }
}

impl ToBytes for ReJoinRequest02 {
    fn to_bytes(&self) -> Vec<u8> {
        let mut ret = Vec::new();
        ret.push(if self.is_type_zero { 0 } else { 2 });

        let mut net_id_reversed = self.net_id;
        net_id_reversed.reverse();
        let mut dev_eui_reversed = *self.dev_eui;
        dev_eui_reversed.reverse();
        
        ret.extend_from_slice(&net_id_reversed);
        ret.extend_from_slice(&dev_eui_reversed);
        ret.extend_from_slice(&self.rj_count0.to_le_bytes());
        ret
    }
}

#[derive(Copy, Clone, Default, Debug)]
pub struct ReJoinRequest1 {
    join_eui: EUI64,
    dev_eui: EUI64,
    rj_count1: u16,
}

impl ReJoinRequest1 {
    pub fn new(join_eui: EUI64, dev_eui: EUI64, rj_count1: u16) -> Self {
        Self {
            join_eui,
            dev_eui,
            rj_count1,
        }
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, LoRaWANError> {
        if bytes.len() != 19 {
            Err(LoRaWANError::InvalidBufferLength)
        }
        else {
            let join_eui_bytes: [u8; 8] = bytes[0..8].try_into()?;
            let dev_eui_bytes: [u8; 8] = bytes[8..16].try_into()?;
            Ok(Self {
                join_eui: EUI64::from(join_eui_bytes),
                dev_eui: EUI64::from(dev_eui_bytes),
                rj_count1: u16::from_le_bytes(bytes[16..18].try_into()?),
            })
        }
    }
}

impl ToBytes for ReJoinRequest1 {
    fn to_bytes(&self) -> Vec<u8> {
        let mut ret = vec![1];
        let mut join_eui_reversed = *self.join_eui;
        join_eui_reversed.reverse(); 
        let mut dev_eui_reversed = *self.dev_eui;
        dev_eui_reversed.reverse();

        ret.extend_from_slice(&join_eui_reversed);
        ret.extend_from_slice(&dev_eui_reversed);
        ret.extend_from_slice(&self.rj_count1.to_le_bytes());
        ret
    }
}