use std::fmt::Debug;

use crate::utils::traits::ToBytes;

#[derive(Copy, Clone, Default, Debug)]
pub struct DownlinkFCtrl {
    //1 byte :: 1 bit for ADR, 1 bit for RFU, 1 bit ACK, 1 bit FPending, 4 bits for FOptsLen
    pub adr: bool,
    //used in ClassB devices to signal that the device has switched to classB mode, ignored in classA
    pub rfu: bool,
    pub ack: bool,
    pub f_pending: bool, //network has more data pending to be sent
    pub f_opts_len: u8,
}

impl DownlinkFCtrl {
    pub fn new(adr:bool, rfu:bool, ack:bool, f_pending: bool, f_opts_len: u8) -> Self {
        Self {
            adr,
            rfu,
            ack,
            f_pending,
            f_opts_len,
        }
    }
}

#[derive(Copy, Clone, Default, Debug)]
pub struct UplinkFCtrl {
    //1 byte :: 1 bit for ADR, 1 bit for RFU, 1 bit ACK, 1 bit ClassB, 4 bits for FOptsLen
    pub adr: bool,
    pub adr_ack_req: bool,
    pub ack: bool,
    pub class_b: bool,
    pub f_opts_len: u8,
}

impl UplinkFCtrl {
    pub fn new(adr:bool, adr_ack_req:bool, ack:bool, class_b: bool, f_opts_len: u8) -> Self {
        Self {
            adr,
            adr_ack_req,
            ack,
            class_b,
            f_opts_len,
        }
    }
}

#[derive(Clone)]
pub enum FCtrl {
    Uplink(UplinkFCtrl),
    Downlink(DownlinkFCtrl)
}

impl Default for FCtrl {
    fn default() -> Self {
        FCtrl::Downlink(DownlinkFCtrl::default())
    }
}

impl Debug for FCtrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FCtrl::Uplink(up) => write!(f, "{up:?}"),
            FCtrl::Downlink(dwn) => write!(f, "{dwn:?}"),
        }
    }
}

impl FCtrl {
    pub fn uplink(fctrl: UplinkFCtrl) -> Self {
        Self::Uplink(fctrl)
    }
    
    pub fn downlink(fctrl: DownlinkFCtrl) -> Self {
        Self::Downlink(fctrl)
    }


    pub fn f_opts_len(&self) -> u8 {
        match self {
            FCtrl::Uplink(up) => {
                up.f_opts_len
            },
            FCtrl::Downlink(dwn) => {
                dwn.f_opts_len
            },
        }
    }
    
    pub(crate) fn set_f_opts_len(&mut self, len: u8) {
        match self {
            FCtrl::Uplink(up) => {
                up.f_opts_len = len
            },
            FCtrl::Downlink(dwn) => {
                dwn.f_opts_len = len
            },
        }
    }

    pub fn is_uplink(&self) -> bool {
        match self {
            FCtrl::Uplink(_) => true,
            FCtrl::Downlink(_) => false,
        }
    }  
    
    pub fn is_downlink(&self) -> bool {
        match self {
            FCtrl::Uplink(_) => false,
            FCtrl::Downlink(_) => true,
        }
    }  

    pub fn is_ack(&self) -> bool {
        match self {
            FCtrl::Uplink(up) => up.ack,
            FCtrl::Downlink(dwn) => dwn.ack,
        }
    }

    pub fn from_bytes(bytes: u8, is_uplink: bool) -> FCtrl {
        if is_uplink {
            let adr =         (bytes & 0b10000000) > 0; 
            let adr_ack_req = (bytes & 0b01000000) > 0; 
            let ack =         (bytes & 0b00100000) > 0; 
            let class_b =     (bytes & 0b00010000) > 0; 
            let f_opts_len =    bytes & 0b00001111; 
            FCtrl::uplink(UplinkFCtrl::new(adr, adr_ack_req, ack, class_b, f_opts_len))
        }
        else {
            let adr =       (bytes & 0b10000000) > 0; 
            let rfu =       (bytes & 0b01000000) > 0; 
            let ack =       (bytes & 0b00100000) > 0; 
            let f_pending = (bytes & 0b00010000) > 0; 
            let f_opts_len =  bytes & 0b00001111; 
            FCtrl::downlink(DownlinkFCtrl::new(adr, rfu, ack, f_pending, f_opts_len))
        }
    }
}

impl ToBytes for FCtrl {
    fn to_bytes(&self) -> Vec<u8> {
        let mut r:u8 = 0b00000000; 
        match self {
            FCtrl::Uplink(uplink) => {
                if uplink.adr         { r |= 0b10000000 };
                if uplink.adr_ack_req { r |= 0b01000000 };
                if uplink.ack         { r |= 0b00100000 };
                if uplink.class_b     { r |= 0b00010000 };
                if uplink.f_opts_len > 0 { r |= uplink.f_opts_len & 0b00001111 }; //last 4 bits
            },
            FCtrl::Downlink(downlink) => {
                if downlink.adr        { r |= 0b10000000 };
                if downlink.rfu        { r |= 0b01000000 };
                if downlink.ack        { r |= 0b00100000 };
                if downlink.f_pending  { r |= 0b00010000 };
                if downlink.f_opts_len > 0 { r |= downlink.f_opts_len & 0b00001111 }; //last 4 bits
            },
        }
        vec![r]
    }
}