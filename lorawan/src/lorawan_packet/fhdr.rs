use std::convert::TryInto;

use crate::{
    device::Device,
    utils::{errors::LoRaWANError, traits::{ToBytes, ToBytesWithContext}}, encryption::aes_128_encrypt_with_padding,
};

use super::fctrl::FCtrl;

#[derive(Default, Debug, Clone)]
//7 to 22 bytes
pub struct FHDR {
    dev_addr: [u8; 4],
    fctrl: FCtrl,
    fcnt: u16,
    fopts: [u8; 15], //encrypted using NwkSEncKey
}

impl FHDR {
    pub fn new(dev_addr: [u8; 4], fctrl: FCtrl) -> Self {
        Self {
            dev_addr,
            fctrl,
            fcnt: 0,
            fopts: [0; 15]
        }
    }
    
    pub fn set_fcnt(&mut self, fcnt: u16) {
        self.fcnt = fcnt
    }
    
    pub fn set_fcnt_from_device(&mut self, device: &Device, fport: Option<u8>) -> Result<(), LoRaWANError> {
        let session_context = device.session().ok_or(LoRaWANError::SessionContextMissing)?;

        self.fcnt = if self.fctrl.is_uplink() {
            session_context.network_context().f_cnt_up()
        } else if fport.is_none() || fport == Some(0) {
            session_context.network_context().nf_cnt_dwn()
        } else {
            session_context.application_context().af_cnt_dwn()
        } as u16;
        Ok(())
    }
    
    /// Get a reference to the fhdr's fctrl.
    pub fn fctrl(&self) -> &FCtrl {
        &self.fctrl
    }

    /// Get the fhdr's fcnt.
    pub fn fcnt(&self) -> u16 {
        self.fcnt
    }

    pub fn dev_addr(&self) -> [u8; 4] {
        self.dev_addr
    }

    pub fn fopts(&self) -> [u8; 15] {
        self.fopts
    }

    pub fn set_fopts(&mut self, fopts: &[u8]) {
        let len = fopts.len();
        let len = if len > 15 { 15 } else { len };
        self.fopts[0..len].copy_from_slice(&fopts[0..len]);
        self.fctrl.set_f_opts_len(len as u8);
    }

    fn encrypt_fopts(&self, device_context: &Device, dev_addr: [u8; 4], is_uplink: bool, fopts: &[u8], f_opts_len: usize) -> Result<Vec<u8>, LoRaWANError> {
        let session_context = device_context.session().ok_or(LoRaWANError::SessionContextMissing)?;              
        let (direction_byte, counter_bytes): (u8, [u8;4]) = if is_uplink { 
            (0, session_context.network_context().f_cnt_up().to_le_bytes()) 
        } else { 
            (1, session_context.network_context().nf_cnt_dwn().to_le_bytes())
        };
        let mut block = vec![
            0x1,
            0, 0, 0, 0,
            direction_byte,
            dev_addr[0], dev_addr[1], dev_addr[2], dev_addr[3],
            counter_bytes[0], counter_bytes[1], counter_bytes[2], counter_bytes[3],
            0, 0
        ];
        let xor_block = aes_128_encrypt_with_padding(session_context.network_context().nwk_s_enc_key(), &mut block)?;
        let (fopts_used, _) = fopts.split_at(f_opts_len);
    
        Ok(fopts_used.iter().zip(xor_block).map(|(v1,v2)| {
            v1 ^ v2
        }).collect())
    }

    //TODO set_fopts from maccommands?
    pub fn from_bytes(bytes: &[u8], device_context: Option<&Device>, is_uplink: bool) -> Result<Self, LoRaWANError> {
        if !(7..=22).contains(&bytes.len()) {
            Err(LoRaWANError::InvalidBufferLength)
        }
        else {
            let mut dev_addr: [u8; 4] = bytes[0..4].try_into()?;
            dev_addr.reverse();

            if let Some(dev) = device_context {
                if let Some(session) = dev.session() {
                    if session.network_context().dev_addr() != &dev_addr {
                        return Err(LoRaWANError::InvalidDevAddr);
                    }
                }
            }

            let fctrl = FCtrl::from_bytes(bytes[4], is_uplink);
            let fcnt: u16 = u16::from_le_bytes(bytes[5..7].try_into()?);
            let mut fopts: [u8; 15] = [0; 15];

            let len: usize = fctrl.f_opts_len().into();
            let fopts_len: usize = if len > 0 {
                if len <= 15 { len } else { 15 }
            } else { 0 };

            fopts[..fopts_len].copy_from_slice(&bytes[7..(7+fopts_len)]);

            let mut fhdr = Self {
                dev_addr,
                fctrl,
                fcnt,
                fopts,
            };

            if let Some(device) = device_context {
                if fopts_len > 0 && device.version().is_1_1_or_greater() {
                    let decrypted_fopts = fhdr.encrypt_fopts(device, dev_addr, is_uplink, &fopts, fopts_len)?;
                    fhdr.set_fopts(&decrypted_fopts)
                }
            } else {/*eprintln!("No device context available, skipping decryption of fopts");*/}
            Ok(fhdr)
        }
    }
}

impl ToBytesWithContext for FHDR {
    fn to_bytes_with_context(&self, device_context: &Device) -> Result<Vec<u8>, LoRaWANError> {
        let mut ret = Vec::new();
        
        let mut dev_addr_reversed = self.dev_addr;
        dev_addr_reversed.reverse();

        ret.extend_from_slice(&dev_addr_reversed);
        ret.extend_from_slice(&self.fctrl.to_bytes());
        ret.extend_from_slice(&self.fcnt.to_le_bytes());

        if self.fctrl.f_opts_len() > 0 {
            if device_context.version().is_1_1_or_greater() {
                let encrypted_fopts: Vec<u8> = self.encrypt_fopts(device_context, self.dev_addr, self.fctrl.is_uplink(), &self.fopts, self.fctrl.f_opts_len().into())?;
                ret.extend_from_slice(&encrypted_fopts);
            } 
            else {
                let (fopts_used, _) = self.fopts.split_at(self.fctrl.f_opts_len().into());
                ret.extend_from_slice(fopts_used)
            }
        }
        Ok(ret)
    }
}