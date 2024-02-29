use std::{ops::{Deref, DerefMut}, cmp::Ordering, time::Duration};
use std::fmt::Debug;
use lorawan::{device::Device, utils::{traits::ToBytes, errors::LoRaWANError}, lorawan_packet::{LoRaWANPacket, payload::Payload, mac_commands::{EDMacCommands, NCMacCommands}}};
use crate::communicator::{LoRaWANCommunicator, CommunicatorError};

pub struct LoRaWANDevice<T> 
where T: LoRaWANCommunicator + Send + Sync {
    device: Device,
    communication: T,
    //config: DeviceConfig,
}

impl<T: LoRaWANCommunicator + Send + Sync> From<LoRaWANDevice<T>> for (Device, T) {
    fn from(val: LoRaWANDevice<T>) -> Self {
        (val.device, val.communication)
    }
}

impl<T> LoRaWANDevice<T> where T: LoRaWANCommunicator + Send + Sync {
    pub fn new(device: Device, communication: T/*, config: DeviceConfig*/) -> Self {
        Self {
            device, communication//, config
        }
    }

    pub fn communicator(&self) -> &T {
        &self.communication
    }
    
    pub fn communicator_mut(&mut self) -> &mut T {
        &mut self.communication
    }

    fn fold_maccomands(fopts: Option<&[EDMacCommands]>) -> Option<Vec<u8>> {
        fopts.map(|mac_commands| {
            mac_commands.iter()
                .fold(Vec::new(),|mut acc, curr| {
                    let curr_slice = curr.to_bytes(); 
                    if acc.len() + curr_slice.len() <= 15 {
                        acc.extend_from_slice(&curr_slice); 
                    }
                    else {
                        eprintln!("{curr:?} is too long to be added to fopts");
                    }
                    acc
                })
            })
    }

    pub async fn send_uplink(&mut self, payload: Option<&[u8]>, confirmed: bool, fport: Option<u8>, fopts: Option<&[EDMacCommands]>) -> Result<(), CommunicatorError> {
        let fopts: Option<Vec<u8>> = LoRaWANDevice::<T>::fold_maccomands(fopts);
        let packet = self.device.create_uplink(payload, confirmed, fport, fopts)?;
        self.communication.send_uplink(&packet, Some(*self.dev_eui()),None).await.unwrap();
        if confirmed {

            //TODO REMOVE PERFORMANCES CHECKS
            //let before = Instant::now();
            let payloads = self.communication.receive_downlink(Some(Duration::from_secs(5))).await.unwrap();
            //let after = Instant::now();
            
            //let mut file = OpenOptions::new()
            //.append(true)
            //.create(true)
            //.open(format!("/root/times_{}.csv", self.dev_eui()))
            //.expect("Failed to open file");
            //writeln!(file, "{},{}", SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis(), (after - before).as_millis()).expect("Error while logging time to file");
            
            //TODO ESTRARRE MEGLIO I PAYLOAD - sf based?
            let (_sf, content) = payloads.iter().next().ok_or(LoRaWANError::MissingDownlink)?;

            let packet = LoRaWANPacket::from_bytes(&content.payload, None, false)?;

            if let Payload::MACPayload(p) = packet.payload() {
                let fcnt = p.fhdr().fcnt();
                let current_fcnt = self.device.session_mut().ok_or(LoRaWANError::ContextNeeded)?.network_context().nf_cnt_dwn(); 
                let (fcnt_valid, fcnt_looped) = Self::nonce_valid(fcnt, current_fcnt as u16);
                let new_fcnt = Self::increment_nonce(fcnt, current_fcnt, fcnt_looped);
                if !fcnt_valid { eprintln!("Invalid {} fcnt down, expected > {current_fcnt}, received {fcnt}", if p.is_application() { "application" } else { "network" }) }
                else if p.is_application() {
                    self.device.session_mut().ok_or(LoRaWANError::ContextNeeded)?.application_context_mut().update_af_cnt_dwn(new_fcnt);
                } else {
                    self.device.session_mut().ok_or(LoRaWANError::ContextNeeded)?.network_context_mut().update_nf_cnt_dwn(new_fcnt);
                }
            };


            let packet = LoRaWANPacket::from_bytes(&content.payload, Some(&self.device), false)?;
            //println!("{packet:?}");
            if let Payload::MACPayload(p) = packet.payload() {
                if let Some(frmp) = p.frm_payload() {
                    match p.fport() {
                        Some(0) | None => {
                            let commands = NCMacCommands::from_bytes(frmp).unwrap();
                            println!("{commands:?}")
                        },
                        Some(_port) => {
                            //println!("Port: {port}, message: {}", String::from_utf8_lossy(frmp));
                        },
                    }
                }
            };
        }
        Ok(())
    }



    pub async fn send_join_request(&mut self) -> Result<(), CommunicatorError> {
        let join_request = self.device.create_join_request()?;
        //println!("{}", PrettyHexSlice(&join_request));
        
        
        self.communication.send_uplink(&join_request, Some(*self.dev_eui()), None).await?;
        let payloads = self.communication.receive_downlink(Some(Duration::from_secs(5))).await?;
        
        //TODO ESTRARRE MEGLIO I PAYLOAD
        let content = payloads.values().next().ok_or(LoRaWANError::MissingDownlink)?;
        //println!("{}", PrettyHexSlice(&content.payload));

        let packet = LoRaWANPacket::from_bytes(&content.payload, Some(&self.device), false)?;
        //println!("{packet:?}");
        
        if let Payload::JoinAccept(ja) = packet.payload() {
            //println!("join accept received: {ja:?}");

            let join_nonce = *ja.join_nonce();
            let current_join_nonce = self.device.join_context().join_nonce();

            let jn_u32 = u32::from_le_bytes([join_nonce[0], join_nonce[1], join_nonce[2], 0]);
            let cjn_u32 = u32::from_le_bytes([current_join_nonce[0], current_join_nonce[1], current_join_nonce[2], 0]);

            //println!("{jn_u32}-{cjn_u32}");
            if cjn_u32 > jn_u32 { 
                eprintln!("Invalid join_nonce, expected > {cjn_u32}, received {jn_u32}"); 
                return Err(CommunicatorError::LoRaWANError(LoRaWANError::InvalidNonce)) 
            }
            else { 
                self.device.join_context_mut().update_join_nonce(jn_u32);
                self.device.generate_session_context(ja)?;
            }
            //println!("{}",self.device);
        }
        Ok(())
    }
    
    pub async fn send_maccommands(&mut self, mac_commands: &[EDMacCommands], confirmed: bool) -> Result<(), CommunicatorError> {        
        let content: Vec<u8> = self.device.create_maccommands(mac_commands)?;
        let uplink = self.device.create_uplink(Some(&content), confirmed, Some(0), None)?;
        self.communication.send_uplink(&uplink, Some(*self.dev_eui()), None).await
    }

    fn nonce_valid(received_nonce: u16, current_nonce: u16) -> (bool, bool) {
        match received_nonce.cmp(&current_nonce) {
            Ordering::Greater => (true, false),
            Ordering::Equal => (false, false),
            Ordering::Less => ((0xffff - current_nonce < 5) && received_nonce < 5, true),
        }
    }
    
    fn increment_nonce(received_nonce: u16, current_nonce: u32, nonce_looped: bool) -> u32 {
        let increment_higher_half_dev_nonce = if nonce_looped { 0x00010000 } else { 0 };
        received_nonce as u32 | ((current_nonce & 0xffff0000) + increment_higher_half_dev_nonce)
    }


    //pub fn extract_config(&self) -> serde_json::Value {
    //    json!({
    //        "device": self.config
    //    })
    //}
}

impl <T> Deref for LoRaWANDevice<T> where T: LoRaWANCommunicator + Send + Sync {
    type Target=Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl <T> DerefMut for LoRaWANDevice<T> where T: LoRaWANCommunicator + Send + Sync {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.device
    }
}

impl <T> Debug for LoRaWANDevice<T> where T: LoRaWANCommunicator + Send + Sync {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoRaWANDevice").field("device", &self.device).finish()
    }
}