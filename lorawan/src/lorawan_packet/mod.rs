use std::convert::TryInto;

use serde::{Deserialize, Serialize};

use crate::{device::{Device, LoRaWANVersion}, utils::{errors::LoRaWANError, traits::{ToBytes, ToBytesWithContext}}, encryption::{self, aes_128_decrypt_with_padding, aes_128_encrypt_with_padding}};

use self::{mac_payload::MACPayload, mhdr::{MHDR, MType}, payload::Payload, join::{JoinAcceptPayload, RejoinRequestPayload, JoinRequestPayload}};
pub mod fctrl;
pub mod fhdr;
pub mod join;
pub mod mac_commands;
pub mod mac_payload;
pub mod payload;
pub mod mhdr;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct LoRaWANPacket {
    mhdr: MHDR,
    payload: Payload,
}

impl LoRaWANPacket {
    pub fn new(mhdr: MHDR, payload: Payload) -> Self {
        Self {
            mhdr,
            payload,
        }
    }

    pub fn set_mhdr(&mut self, mhdr: MHDR) {
        self.mhdr = mhdr;
    }

    /// Get a reference to the lorawanpacket's mhdr.
    pub fn mhdr(&self) -> &MHDR {
        &self.mhdr
    }
    
    pub fn set_payload(&mut self, payload: Payload) {
        self.payload = payload;
    }

    /// Get a reference to the lorawanpacket's payload.
    pub fn payload(&self) -> &Payload {
        &self.payload
    }

    pub fn into_payload(self) -> Payload {
        self.payload
    }

    fn extract_macpayload_mic(payload: &MACPayload, device_context: &Device, full_buffer: &[u8], ) -> Result<[u8;4], LoRaWANError> {
        let session = device_context.session().ok_or(LoRaWANError::SessionContextMissing)?;
        let dev_addr = session.network_context().dev_addr();
        let is_downlink = payload.fhdr().fctrl().is_downlink(); 

        let conf_fcnt: [u8; 2] = if payload.fhdr().fctrl().is_ack() {
            if is_downlink && *device_context.version() == LoRaWANVersion::V1_1 {
                //TODO qui ci va il fnct dell'uplink di cui devi fare l'ack -> come me lo passo? -> probabilmente è l'ultimo uplink ricevuto ->
                session.network_context().f_cnt_up() as u16
            } else if !is_downlink {
                //FIXME se il pacchetto è un uplink in risposta ad un downlink con ack set devo prendere il counter usato in quel pacchetto ma 
                //devo trovare un modo per distinguere se il pacchetto aveva fport == 0 o no, in quel caso devo usare l'application context.
                session.network_context().nf_cnt_dwn() as u16 
            } else {
                0_u16  //downlink if version < 1.1
            }
        } else {
            0_u16
        }.to_le_bytes();

        let txdr_txch: [u8;2] = if is_downlink {
            [0,0]
        } else {
            match device_context.version() {
                LoRaWANVersion::V1_1 => {
                    let tx = 0; //TODO bho prenderli in qualche modo, tocca modellare anche lo strato fisico prima o poi
                    let ch = 0;
                    [tx,ch]
                },
                _ => {
                    [0,0]
                }
            }
        };

        let direction_byte: u8 = u8::from(is_downlink);

        let device_counter_bytes: [u8; 4] = { //get the last two bytes of the counter for context
            if is_downlink {
                if payload.is_application() {
                    session.application_context().af_cnt_dwn()
                } else {
                    session.network_context().nf_cnt_dwn()
                }
            }
            else {
                session.network_context().f_cnt_up()
            }
        }.to_le_bytes();

        let packet_counter_bytes: [u8; 2] = payload.fhdr().fcnt().to_le_bytes();
        let packet_counter = [packet_counter_bytes[0], packet_counter_bytes[1], device_counter_bytes[2], device_counter_bytes[3]];

        let mut block = vec![
            0x49,
            conf_fcnt[0], conf_fcnt[1],
            txdr_txch[0], txdr_txch[1],
            direction_byte,
            dev_addr[3], dev_addr[2], dev_addr[1], dev_addr[0],
            packet_counter[0], packet_counter[1], packet_counter[2], packet_counter[3],
            0, full_buffer.len() as u8
        ];

        block.extend_from_slice(full_buffer);


        let mic = if is_downlink {
            encryption::extract_mic(session.network_context().snwk_s_int_key(), &block)?
        } else {
            match device_context.version() {
                LoRaWANVersion::V1_1 => {
                    let cmac_s = encryption::extract_mic(session.network_context().snwk_s_int_key(), &block)?;
                    
                    block[1] = 0;
                    block[2] = 0;
                    block[3] = 0;
                    block[4] = 0;
                    
                    let cmac_f = encryption::extract_mic(session.network_context().fnwk_s_int_key(), &block)?;
                    [cmac_s[0], cmac_s[1], cmac_f[0], cmac_f[1]]
                },
                _ => {
                    encryption::extract_mic(session.network_context().fnwk_s_int_key(), &block)?
                },
            }
        };

        Ok(mic)
    }

    fn extract_join_request_mic(device_context: &Device, full_buffer: &[u8]) -> Result<[u8;4], LoRaWANError> {
        let key = device_context.network_key();
        encryption::extract_mic(key, full_buffer)
    }

    fn extract_join_accept_mic(join_accept: &JoinAcceptPayload, device_context: &Device, full_buffer: &[u8]) -> Result<[u8;4], LoRaWANError> {
        let mic = if join_accept.opt_neg() {
            let key = device_context.join_context().js_int_key();
            let j_req_type = join_accept.join_req_type().to_byte();
            let join_eui = **device_context.join_eui();
            let dev_nonce: [u8; 2] = (device_context.dev_nonce() as u16).to_be_bytes();
            
            let mut buffer = vec![j_req_type];
            buffer.extend_from_slice(&join_eui);
            buffer.extend_from_slice(&dev_nonce);
            buffer.extend_from_slice(full_buffer);
            encryption::extract_mic(key, &buffer)?
        } else {
            let key = device_context.network_key();
            encryption::extract_mic(key, full_buffer)?
        };
        Ok(mic)
    }
    
    fn extract_rejoin_request_mic(rejoin_request: &RejoinRequestPayload, device_context: &Device, full_buffer: &[u8]) -> Result<[u8;4], LoRaWANError> { 
        let key = match rejoin_request {
            RejoinRequestPayload::T1(_) => device_context.join_context().js_int_key(),
            RejoinRequestPayload::T02(_) => device_context.session().ok_or(LoRaWANError::SessionContextMissing)?.network_context().snwk_s_int_key(),
        };
        encryption::extract_mic(key, full_buffer)
    }

    fn extract_mic(&self, device_context: &Device, full_buffer: &[u8]) -> Result<[u8;4], LoRaWANError> {
        match &self.payload {
            Payload::JoinRequest(_) => LoRaWANPacket::extract_join_request_mic(device_context, full_buffer),
            Payload::RejoinRequest(rj) => LoRaWANPacket::extract_rejoin_request_mic(rj, device_context, full_buffer),
            Payload::JoinAccept(ja) => LoRaWANPacket::extract_join_accept_mic(ja, device_context, full_buffer), //teoricamente non esiste
            Payload::MACPayload(mp) => LoRaWANPacket::extract_macpayload_mic(mp, device_context, full_buffer),
            //Payload::Proprietary(_) => if let Some(handlers) = device_context.proprietary_payload_handlers() {
            //    handlers.custom_mic_function()(full_buffer)
            //} else {
            //    Ok([0,0,0,0])
            //}
            Payload::Proprietary(_) => Ok([0,0,0,0])
        }
    }

    fn check_coherence_mhdr_fctrl(&self) -> Result<(), LoRaWANError> {
        let mtype = self.mhdr.mtype();
        let mhdr_payload_coherence = match self.payload {
            Payload::JoinRequest(_) => mtype == MType::JoinRequest,
            Payload::JoinAccept(_) => mtype == MType::JoinAccept,
            Payload::RejoinRequest(_) => mtype == MType::RejoinRequest,
            Payload::Proprietary(_) => mtype == MType::Proprietary,
            Payload::MACPayload(_) => !(mtype == MType::JoinRequest || mtype == MType::JoinAccept || mtype == MType::RejoinRequest || mtype == MType::Proprietary),
        };
        if !mhdr_payload_coherence { Err(LoRaWANError::MHDRNotCoherentWithPayload) }
        else if mtype == MType::Proprietary { Ok(()) }
        else {
            let is_uplink_payload = match &self.payload {
                Payload::JoinRequest(_) => true,
                Payload::JoinAccept(_) => false,
                Payload::RejoinRequest(_) => true,
                Payload::MACPayload(p) => p.fhdr().fctrl().is_uplink(),
                Payload::Proprietary(_) => true,
            };

            let is_uplink_mtype = match mtype {
                MType::JoinRequest | 
                MType::UnconfirmedDataUp |
                MType::ConfirmedDataUp | 
                MType::RejoinRequest => true,

                MType::JoinAccept |
                MType::UnconfirmedDataDown |
                MType::ConfirmedDataDown => false,

                MType::Proprietary => true,
            };
            if is_uplink_mtype && is_uplink_payload || !is_uplink_mtype && !is_uplink_payload {
                Ok(())
            }
            else { Err(LoRaWANError::FCtrlNotCoherentWithPayload) }
        }
    }

    pub fn validate_mic(buffer: &[u8], packet: &LoRaWANPacket, device_context: &Device) -> Result<(), LoRaWANError> {
        let len = buffer.len();
        if len < 12 {
            return Err(LoRaWANError::InvalidBufferLength);
        }
        let (packet_bytes, mic) = buffer.split_at(len - 4);
        let mic: [u8; 4] = mic.try_into()?;
        let expected_mic = packet.extract_mic(device_context, packet_bytes)?;
        if mic != expected_mic { Err(LoRaWANError::InvalidMic) } else { Ok(()) }
    }

    pub fn from_bytes(bytes: &[u8], device_context: Option<&Device>, is_uplink: bool) -> Result<Self, LoRaWANError> {
        let len = bytes.len(); 
        if len < 12 {
            Err(LoRaWANError::InvalidBufferLength)
        }
        else {
            let mhdr = MHDR::from_bytes(bytes[0]);

            let packet_and_context_coherent = match mhdr.mtype() {
                MType::JoinRequest         | 
                MType::RejoinRequest       |
                MType::UnconfirmedDataUp   |
                MType::ConfirmedDataUp     => is_uplink,
                
                MType::JoinAccept          |
                MType::UnconfirmedDataDown |
                MType::ConfirmedDataDown   => !is_uplink,
                MType::Proprietary         => true,
            };

            if !packet_and_context_coherent {
                return Err(LoRaWANError::MHDRNotCoherentWithContext);
            }

            let payload = match mhdr.mtype() {
                MType::JoinRequest => {
                    Payload::JoinRequest(JoinRequestPayload::from_bytes(&bytes[1..len-4])?)
                },
                MType::JoinAccept => {
                    let mut payload_bytes = bytes[1..].to_vec();
                    if let Some(device) = device_context {
                        let key = if device.last_join_request_received().is_rejoin() {
                            device.join_context().js_enc_key()
                        } else {
                            device.network_key()
                        };
    
                        let decrypted = aes_128_encrypt_with_padding(key, &mut payload_bytes)?;
                        let len = decrypted.len();
                        let (decrypted_payload_bytes, mic) = decrypted.split_at(len - 4);
    
                        let mut mic_buffer = vec![bytes[0]];
                        mic_buffer.extend_from_slice(decrypted_payload_bytes);
    
                        let join_accept_payload = JoinAcceptPayload::from_bytes(decrypted_payload_bytes, device.last_join_request_received())?;
                        let expected_mic = LoRaWANPacket::extract_join_accept_mic(&join_accept_payload, device, &mic_buffer)?;
                        if expected_mic != mic {
                            return Err(LoRaWANError::InvalidMic);
                        }
                        Payload::JoinAccept(join_accept_payload)
                    } else {
                        //println!("No device context, skipping JoinAccept decryption and mic validation");
                        let len =  payload_bytes.len();
                        let (payload_bytes, _) = payload_bytes.split_at(len - 4);
                        let join_accept_payload = JoinAcceptPayload::from_bytes(payload_bytes, &join::JoinRequestType::JoinRequest)?;
                        Payload::JoinAccept(join_accept_payload)
                    }
                },
                MType::RejoinRequest => {
                    Payload::RejoinRequest(RejoinRequestPayload::from_bytes(&bytes[1..len-4])?)
                },
                MType::Proprietary => {
                    Payload::Proprietary(Vec::from(&bytes[1..]))
                },
                _ => {
                    Payload::MACPayload(MACPayload::from_bytes(&bytes[1..len-4], device_context, is_uplink)?)
                },
            };

            let packet = LoRaWANPacket::new(mhdr, payload);
            packet.check_coherence_mhdr_fctrl()?;
            if MType::JoinAccept != packet.mhdr.mtype() { //join accept MIC is already handled
                if let Some(device) = device_context {
                    LoRaWANPacket::validate_mic(bytes, &packet, device)?;
                }
                else {
                    //println!("No device context, skipping mic validation");
                }
            };
            Ok(packet)
             
        }
    }

    pub fn is_join_request(&self) -> bool {
        matches!(self.mhdr.mtype(), MType::JoinRequest)
    }

    pub fn extract_mtype(first_byte: u8) -> MType {
        MHDR::from_bytes(first_byte).mtype()
    }

}

impl ToBytesWithContext for LoRaWANPacket {
    fn to_bytes_with_context(&self, device_context: &Device) -> Result<Vec<u8>, LoRaWANError> {
        self.check_coherence_mhdr_fctrl()?;
        let mut ret: Vec<u8> = Vec::new();
        ret.extend_from_slice(&self.mhdr.to_bytes());
        match &self.payload {
            Payload::JoinAccept(ja) => {
                //nella join accept il mic si calcola prima e poi si cripta tutto quanto
                let mut payload_mic_buffer = ret.clone();
                payload_mic_buffer.append(&mut self.payload.to_bytes_with_context(device_context)?);
                payload_mic_buffer.extend_from_slice(&self.extract_mic(device_context, &payload_mic_buffer)?);
                payload_mic_buffer.remove(0); //remove the mhdr for encryption

                let key = if ja.is_rejoin() {
                    device_context.join_context().js_enc_key()
                } else {
                    device_context.network_key()
                };
                ret.extend_from_slice(&aes_128_decrypt_with_padding(key, &mut payload_mic_buffer)?);
            },
            _ => {
                ret.extend_from_slice(&self.payload.to_bytes_with_context(device_context)?);
                ret.extend_from_slice(&self.extract_mic(device_context, &ret)?);
            },
        }
        Ok(ret)
    }
}