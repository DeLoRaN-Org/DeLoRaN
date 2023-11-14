use crate::{device::Device, utils::{errors::LoRaWANError, traits::ToBytesWithContext, self}, encryption::{key::Key, aes_128_encrypt_with_padding}};
use super::fhdr::FHDR;

#[derive(Default, Debug, Clone)]
pub struct MACPayload {
    //at least 7 bytes -- Max len is region specific and specified in chapter 6
    fhdr: FHDR,
    fport: Option<u8>, //optional, ranges from 1-223 (0x01, 0x0DF), port 224 is reserved for MAC layer test protocol. Larger values should be discarded cause they are reserved for future use.
    frm_payload: Option<Vec<u8>>, //optional, long N bytes where N is region specific. N must be <= M - 1 (len(fport)) - (len(FHDR)) where M is the maximum MACPayload len.
}

impl MACPayload {
    pub fn new(fhdr: FHDR, fport: Option<u8>, frm_payload: Option<Vec<u8>>) -> Self {
        Self {
            fhdr,
            fport,
            frm_payload,
        }
    }

    fn encrypt_payload(key: &Key, payload: &mut Vec<u8>, dev_addr: &[u8;4], direction_byte: u8, counter: u32) -> Result<(), LoRaWANError> {
        const CHUNK_SIZE: usize = 16;
        let len = payload.len();
        utils::pad_to_16(payload);
        let packet_counter: [u8; 4] = counter.to_le_bytes();

        for (index, chunk) in payload.chunks_mut(CHUNK_SIZE).enumerate() {
            let mut block = vec![
                0x1, 
                0, 0, 0, 0, 
                direction_byte,
                dev_addr[3], dev_addr[2], dev_addr[1], dev_addr[0],
                packet_counter[0], packet_counter[1], packet_counter[2], packet_counter[3],
                0 , (index + 1) as u8
            ];
            
            let enc_block= aes_128_encrypt_with_padding(key, &mut block)?;
            for (b1,b2) in chunk.iter_mut().zip(enc_block.iter())  {
                *b1 ^= *b2;
            }
        }
        payload.truncate(len);
        Ok(())
    }

    /// Get a reference to the macpayload's fhdr.
    pub fn fhdr(&self) -> &FHDR {
        &self.fhdr
    }

    /// Get the macpayload's fport.
    pub fn fport(&self) -> Option<u8> {
        self.fport
    }
    
    pub fn is_application(&self) -> bool {
        self.fport.unwrap_or(0) != 0
    }

    /// Set the macpayload's fhdr.
    pub fn set_fhdr(&mut self, fhdr: FHDR) {
        self.fhdr = fhdr;
    }

    /// Set the macpayload's fport.
    pub fn set_fport(&mut self, fport: Option<u8>) {
        self.fport = fport;
    }

    /// Set the macpayload's frm payload.
    pub fn set_frm_payload(&mut self, frm_payload: Option<Vec<u8>>) {
        self.frm_payload = frm_payload;
    }
    
    /// Get the macpayload's frm payload.
    pub fn frm_payload(&self) -> Option<&Vec<u8>> {
        self.frm_payload.as_ref()
    }

    pub fn from_bytes(bytes: &[u8], device_context: Option<&Device>, is_uplink: bool) -> Result<Self, LoRaWANError> {
        let buffer_len = bytes.len();
        let min_fhdr_len = 7;
        if buffer_len < min_fhdr_len {
            Err(LoRaWANError::InvalidBufferLength)
        }
        else {
            let fopts_len = (bytes[4] & 0b00001111) as usize;
            let fhdr_len = min_fhdr_len + fopts_len; 
            if buffer_len < (min_fhdr_len + fopts_len) {
                Err(LoRaWANError::InvalidBufferLength)
            }
            else {
                let fhdr = FHDR::from_bytes(&bytes[0..fhdr_len], device_context, is_uplink)?;
                let (fport, frm_payload) = if buffer_len > min_fhdr_len + fopts_len {
                    let fport = bytes[fhdr_len];
                    let mut decrypted_payload = Vec::from(&bytes[fhdr_len + 1..]);
                    if let Some(device) = device_context {
                        let session_context = device.session().ok_or(LoRaWANError::SessionContextMissing)?;
                        let key = if fport == 0 {
                            session_context.network_context().nwk_s_enc_key()
                        } else {
                            session_context.application_context().app_s_key()
                        };
                        let direction_byte = if is_uplink { 0 } else { 1 };
    
                        //TODO ma come fanno i cli vari a scomporre il pacchetto senza avere il contesto del pacchetto?
                        //solo con il fcnt dell'fhdr? mancano i primi 2 bytes!! -> in realtà pare con un for a caso -> https://vscode.dev/github/TheThingsNetwork/lorawan-stack/cmd/ttn-lw-cli/commands/lorawan.go line 81
                        // --> studiarlo dalle varie implementazioni
                        /*let counter = if direction_byte == 0 {
                            session_context.network_context().f_cnt_up()
                        } else if fport == 0 {
                            session_context.network_context().nf_cnt_dwn()
                        } else {
                            session_context.application_context().af_cnt_dwn()
                        };*/
                        let counter = fhdr.fcnt() as u32;
                        MACPayload::encrypt_payload(key, &mut decrypted_payload, session_context.network_context().dev_addr(), direction_byte, counter)?;
                    } else {
                        //println!("No device context, skipping MACPayload decryption");
                    }
                    (Some(fport), Some(decrypted_payload))
                } else if buffer_len == min_fhdr_len + fopts_len + 1 {
                    (Some(bytes[fhdr_len]), None)
                } else {
                    (None, None)
                };
                Ok(Self {
                    fhdr,
                    fport,
                    frm_payload
                })
            }
        }
    }
}


impl ToBytesWithContext for MACPayload {
    fn to_bytes_with_context(&self, device_context: &Device) -> Result<Vec<u8>, LoRaWANError> {
        let mut ret = Vec::with_capacity(64);

        if  (!self.is_application() && self.fhdr.fctrl().f_opts_len() > 0) ||
            (self.fport.is_some() && self.frm_payload.is_none()) || 
            (self.fport.is_none() && self.frm_payload.is_some()) {
            return Err(LoRaWANError::FPortInvalidValue);
        }
        ret.extend_from_slice(&self.fhdr.to_bytes_with_context(device_context)?);

        if let Some(fport) = self.fport {
            ret.push(fport);
            if let Some(payload) = &self.frm_payload {
                let session_context = device_context.session().ok_or(LoRaWANError::SessionContextMissing)?;
                
                let direction_byte: u8 = if self.fhdr.fctrl().is_uplink() { 0 } else { 1 }; 
                let dev_addr = session_context.network_context().dev_addr();
                
                let packet_counter = if self.fhdr.fctrl().is_uplink() {
                    session_context.network_context().f_cnt_up()
                } else if fport == 0 {
                    session_context.network_context().nf_cnt_dwn()
                } else {
                    session_context.application_context().af_cnt_dwn()
                };

                let encryption_key = if fport == 0 {
                    session_context.network_context().nwk_s_enc_key()
                } else {
                    session_context.application_context().app_s_key()
                };

                let mut cloned_payload = payload.clone();
                MACPayload::encrypt_payload(encryption_key, &mut cloned_payload, dev_addr, direction_byte, packet_counter)?;

                ret.extend_from_slice(&cloned_payload);
            }
        }
        Ok(ret)
    }
}