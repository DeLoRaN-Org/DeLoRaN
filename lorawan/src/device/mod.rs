pub mod session_context;
pub mod proprietary_payload_handlers;

use std::fmt::Display;

use serde::{Serialize, Deserialize};

use crate::{
    encryption::key::Key,
    lorawan_packet::{join::{JoinAcceptPayload, JoinRequestType, JoinRequestPayload}, mhdr::{MHDR, MType, Major}, payload::Payload, LoRaWANPacket, fctrl::{FCtrl, UplinkFCtrl}, fhdr::FHDR, mac_commands::EDMacCommands, mac_payload::MACPayload},
    utils::{errors::LoRaWANError, eui::EUI64, traits::{ToBytes, ToBytesWithContext}},
    device::session_context::{JoinSessionContext, SessionContext}, regional_parameters::region::RegionalParameters
};


#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default, Hash)]
pub enum DeviceClass {
    #[default] A,
    B,
    C,
}

impl DeviceClass {
    pub fn to_byte(&self) -> u8 {
        match self {
            DeviceClass::A => 0x01,
            DeviceClass::B => 0x02,
            DeviceClass::C => 0x03,
        }
    }
    
    pub fn from_byte(b: u8) -> Self {
        match b {
            0x01 => DeviceClass::A,
            0x03 => DeviceClass::C,
            _ =>    DeviceClass::B,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum LoRaWANVersion {
    V1_0,
    V1_0_1,
    V1_0_2,
    V1_0_3,
    V1_0_4,
    V1_1,
}

impl LoRaWANVersion {
    pub fn is_1_1_or_greater(&self) -> bool {
        !matches!(self, LoRaWANVersion::V1_0 |
            LoRaWANVersion::V1_0_1 |
            LoRaWANVersion::V1_0_2 |
            LoRaWANVersion::V1_0_3 |
            LoRaWANVersion::V1_0_4)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Hash)]
pub enum ActivationMode {
    ABP,
    OTAA,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Device {
    class: DeviceClass,
    version: LoRaWANVersion,
    activation_mode: ActivationMode,
    dev_nonce: u32,
    
    dev_eui: EUI64,
    join_eui: EUI64,

    nwk_key: Key,
    app_key: Key,
    
    join_context: JoinSessionContext,
    session: Option<SessionContext>,

    //proprietary_payload_handlers: Option<ProprietaryPayloadHandlers>,
    last_join_request_received: JoinRequestType,
    regional_params: Option<RegionalParameters>,
}

#[allow(clippy::too_many_arguments)]
impl Device {
    pub fn new(
        class: DeviceClass,
        regional_params: Option<RegionalParameters>,
        dev_eui: EUI64,
        join_eui: EUI64,
        nwk_key: Key,
        app_key: Key,
        version: LoRaWANVersion,
        //proprietary_payload_handlers: Option<ProprietaryPayloadHandlers>
    ) -> Self {
        Self { 
            class,
            regional_params,
            activation_mode: ActivationMode::OTAA,
            dev_nonce: 0,
            dev_eui,
            join_eui,
            nwk_key,
            app_key,
            session: None,
            join_context: JoinSessionContext::derive(&nwk_key, &dev_eui).unwrap(), //FIXME vorrei evitare l'unwrap ma anche che il new possa ritornare errore visto che non può
            version,
            //proprietary_payload_handlers,
            last_join_request_received: JoinRequestType::JoinRequest,
        }
    }
    

    pub fn generate_session_context(&mut self, join_accept_payload: &JoinAcceptPayload) -> Result<(), LoRaWANError> {
        self.session = Some(SessionContext::derive(
            join_accept_payload.opt_neg(),
            &self.nwk_key,
            &self.app_key,
            join_accept_payload.join_nonce(),
            self.join_eui,
            self.dev_nonce,
            join_accept_payload.dev_addr(),
            join_accept_payload.home_net_id(),
        )?);
        Ok(())
    }

    pub fn set_activation_abp(&mut self, session: SessionContext) {
        self.activation_mode = ActivationMode::ABP;
        self.session = Some(session);
    }

    pub fn is_otaa(&self) -> bool {
        self.activation_mode == ActivationMode::OTAA
    }

    pub fn set_dev_nonce(&mut self, dev_nonce: u32) {
        self.dev_nonce = dev_nonce;
    }

    pub fn session(&self) -> Option<&SessionContext> {
        self.session.as_ref()
    }

    pub fn session_mut(&mut self) -> Option<&mut SessionContext> {
        self.session.as_mut()
    }

    pub fn network_key(&self) -> &Key {
        &self.nwk_key
    }

    pub fn app_key(&self) -> &Key {
        &self.app_key
    }

    pub fn dev_nonce_autoinc(&mut self) -> u32 {
        self.dev_nonce += 1;
        self.dev_nonce
    }

    pub fn dev_nonce(&self) -> u32 {
        self.dev_nonce
    }

    pub fn join_context(&self) -> &JoinSessionContext {
        &self.join_context
    }
    
    pub fn join_context_mut(&mut self) -> &mut JoinSessionContext {
        &mut self.join_context
    }

    /// Get a reference to the device's version.
    pub fn version(&self) -> &LoRaWANVersion {
        &self.version
    }

    /// Get the device's dev eui.
    pub fn dev_eui(&self) -> &EUI64 {
        &self.dev_eui
    }

    /// Get a reference to the device's class.
    pub fn class(&self) -> &DeviceClass {
        &self.class
    }

    /// Get a reference to the device's region.
    pub fn regional_parameters(&self) -> &Option<RegionalParameters> {
        &self.regional_params
    }

    /// Get the device's join eui.
    pub fn join_eui(&self) -> &EUI64 {
        &self.join_eui
    }

    /// Get a reference to the device's proprietary payload.
    //pub fn proprietary_payload_handlers(&self) -> Option<&ProprietaryPayloadHandlers> {
    //    self.proprietary_payload_handlers.as_ref()
    //}

    pub fn is_initialized(&self) -> bool {
        self.session.is_some()
    }

    /// Get a reference to the device's last join request received.
    pub fn last_join_request_received(&self) -> &JoinRequestType {
        &self.last_join_request_received
    }

    /// Set the device's last join request received.
    pub fn set_last_join_request_received(&mut self, last_join_request_received: JoinRequestType) {
        self.last_join_request_received = last_join_request_received;
    }

    pub fn create_join_request(&mut self) -> Result<Vec<u8>, LoRaWANError> {
        let mhdr = MHDR::new(MType::JoinRequest, Major::R1);
        let payload = JoinRequestPayload::new(
            self.join_eui,
            self.dev_eui,
            (self.dev_nonce_autoinc()) as u16,
        );
        let packet = LoRaWANPacket::new(mhdr, Payload::JoinRequest(payload));
        packet.to_bytes_with_context(self)
    }

    pub fn create_uplink(&mut self, payload: Option<&[u8]>, confirmed: bool, fport: Option<u8>, fopts: Option<Vec<u8>>) -> Result<Vec<u8>, LoRaWANError> {
        let mtype = if confirmed {
            MType::ConfirmedDataUp
        } else {
            MType::UnconfirmedDataUp
        };
        let mhdr = MHDR::new(mtype, Major::R1);
        
        let (f_opts_len, fopts) = match fopts {
            Some(mut v) => {
                let len = if v.len() > 15 { 15 } else { v.len()};
                if len == 0 {
                    (0, None)
                } else {
                    v.truncate(len);
                    (len as u8, Some(v))
                }
            }
            None => (0, None),
        };
        
        let session_context = self.session.as_mut().ok_or(LoRaWANError::SessionContextMissing)?;
        let fctrl = FCtrl::Uplink(UplinkFCtrl::new(true, false, true, false, f_opts_len));
        let fcnt = session_context.network_context_mut().f_cnt_up_autoinc() as u16;
        
        let mut fhdr = FHDR::new(*self.session.as_ref().ok_or(LoRaWANError::SessionContextMissing)?.network_context().dev_addr(), fctrl);
        fhdr.set_fcnt(fcnt);

        
        if let Some(f_opts) = fopts {
            fhdr.set_fopts(&f_opts)
        }
        
        let payload = Payload::MACPayload(MACPayload::new(fhdr, fport, payload.map(Vec::from)));

        let packet = LoRaWANPacket::new(mhdr, payload);
        //println!("{packet:?}");
        packet.to_bytes_with_context(self)
    }
    
    pub fn create_maccommands(&mut self, mac_commands: &[EDMacCommands]) -> Result<Vec<u8>, LoRaWANError> {        
        return Ok(mac_commands.iter()
            .map(|e| e.to_bytes())
            .reduce(|mut acc, curr| {acc.extend(&curr); acc}).unwrap());
    }
}

impl Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Device: {{")?;
        writeln!(f, "    class: {:?}", self.class)?;
        writeln!(f, "    Version: {:?}", self.version)?;
        writeln!(f, "    Radio Config: {:?}", self.regional_params)?;
        writeln!(f, "    Activation mode: {:?}", self.activation_mode)?;
        writeln!(f, "    DevNonce: {:?}", self.dev_nonce)?;
        writeln!(f, "    DevEUI: {}", self.dev_eui)?;
        writeln!(f, "    JoinEUI: {}", self.join_eui)?;
        writeln!(f, "    NwkKey: {}", self.nwk_key)?;
        writeln!(f, "    AppKey: {}", self.app_key)?;
        writeln!(f, "    JoinContext: {}", self.join_context)?;
        writeln!(f, "    SessionContext: {}", if self.session.is_some() { self.session.as_ref().unwrap().to_string() } else { "Not initialized".to_string() })?;
        //writeln!(f, "    ProprietaryPayloadHandler: {:?}", self.proprietary_payload_handlers)?;
        writeln!(f, "    LastJoinRequestReceived: {:?}", self.last_join_request_received)?;
        writeln!(f, "}}")?;
        Ok(())
    }
}