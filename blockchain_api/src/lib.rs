pub mod exec_bridge;
pub mod http_bridge;
pub mod mock_bridge;
pub mod convergence;
pub mod udp_bridge;


use std::fmt::Display;
use std::net::IpAddr;
use lorawan::device::session_context::{
    ApplicationSessionContext, NetworkSessionContext, SessionContext,
};
use lorawan::device::{ActivationMode, Device, DeviceClass, LoRaWANVersion};
use lorawan::encryption::key::Key;
use lorawan::lorawan_packet::join::JoinRequestType;
use lorawan::regional_parameters::region::Region;
use lorawan::utils::eui::EUI64;
use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize)]
pub struct BlockchainDeviceConfig {
    pub activation_mode: ActivationMode,
    pub app_key: Key,
    pub class: DeviceClass,
    pub dev_addr: Option<[u8; 4]>,
    pub dev_eui: EUI64,
    pub dev_nonce: u32,
    pub join_eui: EUI64,
    pub join_nonce: u32,
    pub js_enc_key: Key,
    pub js_int_key: Key,
    pub last_join_request_received: JoinRequestType,
    pub nwk_key: Key,
    pub owner: String,
    pub region: Region,
    pub rj_count1: u16,
    pub version: LoRaWANVersion,
}

impl From<BlockchainDeviceConfig> for Device {
    fn from(c: BlockchainDeviceConfig) -> Self {
        let mut d = Device::new(
            c.class, None, c.dev_eui, c.join_eui, c.nwk_key, c.app_key, c.version,
        );
        d.set_dev_nonce(c.dev_nonce);
        d.set_last_join_request_received(c.last_join_request_received);
        d
    }
}

impl From<&Device> for BlockchainDeviceConfig {
    fn from(d: &Device) -> Self {
        let jn = d.join_context().join_nonce();
        Self {
            class: *d.class(),
            version: *d.version(),
            region: *d.regional_parameters().unwrap().region(),
            activation_mode: if d.is_otaa() { ActivationMode::OTAA} else { ActivationMode::ABP },
            dev_nonce: d.dev_nonce(),
            dev_eui: *d.dev_eui(),
            join_eui: *d.join_eui(),
            nwk_key: *d.network_key(),
            app_key: *d.app_key(),
            js_int_key: *d.join_context().js_int_key(),
            js_enc_key: *d.join_context().js_enc_key(),
            rj_count1: d.join_context().rj_count1(),
            join_nonce: u32::from_le_bytes([0, jn[2], jn[1], jn[0]]),
            last_join_request_received: *d.last_join_request_received(),
            dev_addr: d.session().map(|s| *s.network_context().dev_addr()),
            owner: "owner".to_string(), //TODO AGGIUSTARE QUESTA COSA(?)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockchainDeviceSession {
    pub af_cnt_dwn: u32,
    pub app_s_key: Key,
    pub dev_addr: [u8; 4],
    pub dev_eui: EUI64,
    pub f_cnt_up: u32,
    pub fnwk_s_int_key: Key,
    pub home_net_id: [u8; 3],
    pub nf_cnt_dwn: u32,
    pub nwk_s_enc_key: Key,
    pub owner: String,
    pub rj_count0: u16,
    pub snwk_s_int_key: Key,
    pub nc_ids: Vec<String>,
}

impl From<BlockchainDeviceSession> for SessionContext {
    fn from(bds: BlockchainDeviceSession) -> Self {
        SessionContext::new(
            ApplicationSessionContext::new(bds.app_s_key, bds.af_cnt_dwn),
            NetworkSessionContext::new(
                bds.fnwk_s_int_key,
                bds.snwk_s_int_key,
                bds.nwk_s_enc_key,
                bds.home_net_id,
                bds.dev_addr,
                bds.f_cnt_up,
                bds.nf_cnt_dwn,
                bds.rj_count0,
            ),
        )
    }
}

impl BlockchainDeviceSession {
    pub fn from(s: &SessionContext, dev_eui: &EUI64) -> Self {
        BlockchainDeviceSession {
            fnwk_s_int_key: *s.network_context().fnwk_s_int_key(),
            snwk_s_int_key: *s.network_context().snwk_s_int_key(),
            nwk_s_enc_key: *s.network_context().nwk_s_enc_key(),
            home_net_id: s.network_context().home_net_id(),
            dev_addr: *s.network_context().dev_addr(),
            f_cnt_up: s.network_context().f_cnt_up(),
            nf_cnt_dwn: s.network_context().nf_cnt_dwn(),
            rj_count0: s.network_context().rj_count0(),
            app_s_key: *s.application_context().app_s_key(),
            af_cnt_dwn: s.application_context().af_cnt_dwn(),
            dev_eui: *dev_eui,
            owner: "".to_owned(), //TODO unknown
            nc_ids: vec![],       //TODO unknown
        }
    }
}

impl From<BlockchainDeviceSession> for Device {
    fn from(bds: BlockchainDeviceSession) -> Self {
        let mut d = Device::new(
            DeviceClass::default(),
            None,
            bds.dev_eui,
            EUI64::default(),
            Key::default(),
            Key::default(),
            LoRaWANVersion::V1_0_4,
        );
        d.set_activation_abp(bds.into());
        d
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockchainPacket {
    pub hash: String,
    pub timestamp: String,
    pub dev_id: String,
    pub length: u32, //
    //calculate time-on-air of the packet
    pub sf: u16, //
    pub gws: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockchainPrivatePacket {
    pub hash: String,
    pub packet: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockchainState {
    pub configs: Vec<BlockchainDeviceConfig>,
    pub packets: Vec<BlockchainPacket>,
}

#[derive(Debug)]
pub enum BlockchainError {
    GenericError(String),
    Error(&'static str),
    MissingContent,
    JSONParsingError,
}

impl Display for BlockchainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockchainError::Error(e) => write!(f, "BE::Error: {}", e),
            BlockchainError::GenericError(e) => write!(f, "BE::GenericError: {}", e),
            BlockchainError::MissingContent => write!(f, "BE::MissingContent"),
            BlockchainError::JSONParsingError => write!(f, "BE::JSONParsingError"),
        }
    }
}


pub trait BlockchainClient: Send + Sync {
    type Config: Send + Sync + Clone;

    fn from_config(config: &Self::Config) -> impl std::future::Future<Output = Result<Box<Self>, BlockchainError>> + Send;
    fn get_hash(&self) -> impl std::future::Future<Output = Result<String, BlockchainError>> + Send;
    fn get_device_session(&self,    dev_addr: &[u8; 4]) -> impl std::future::Future<Output = Result<BlockchainDeviceSession, BlockchainError>> + Send;
    fn get_device_config(&self, dev_eui: &EUI64,) -> impl std::future::Future<Output = Result<BlockchainDeviceConfig, BlockchainError>> + Send;
    fn get_device(&self, dev_eui: &EUI64) -> impl std::future::Future<Output = Result<Device, BlockchainError>> + Send;
    fn get_all_devices(&self) -> impl std::future::Future<Output = Result<BlockchainState, BlockchainError>> + Send;
    fn create_device_config(&self, device: &Device) -> impl std::future::Future<Output = Result<(), BlockchainError>> + Send;
    fn delete_device(&self, dev_eui: &EUI64) -> impl std::future::Future<Output = Result<(), BlockchainError>> + Send;
    fn delete_device_session(&self, dev_addr: &[u8; 4]) -> impl std::future::Future<Output = Result<(), BlockchainError>> + Send;
    fn create_uplink(&self, packet: &[u8], answer: Option<&[u8]>) -> impl std::future::Future<Output = Result<(),BlockchainError>> + Send;
    fn join_procedure(&self, join_request: &[u8], join_accept: &[u8], nc_id: &str, dev_id: &EUI64) -> impl std::future::Future<Output = Result<bool,BlockchainError>> + Send;
    fn get_packet(&self, hash: &str) -> impl std::future::Future<Output = Result<BlockchainPacket,BlockchainError>> + Send;
    fn get_public_blockchain_state(&self) -> impl std::future::Future<Output = Result<BlockchainState, BlockchainError>> + Send;
    fn get_device_org(&self, dev_id: &[u8]) -> impl std::future::Future<Output = Result<String, BlockchainError>> + Send;
    fn get_org_anchor_address(&self, org: &str) -> impl std::future::Future<Output = Result<(IpAddr, u16), BlockchainError>> + Send;
}


//TODO implement sync versions where possible
pub trait BlockchainClientSync: Send + Sync {
    type Config: Send + Sync + Clone;

    fn from_config(config: &Self::Config) -> Result<Box<Self>, BlockchainError>;
    fn get_hash(&self) -> Result<String, BlockchainError>;
    fn get_device_session(&self,    dev_addr: &[u8; 4]) -> Result<BlockchainDeviceSession, BlockchainError>;
    fn get_device_config(&self, dev_eui: &EUI64,) -> Result<BlockchainDeviceConfig, BlockchainError>;
    fn get_device(&self, dev_eui: &EUI64) -> Result<Device, BlockchainError>;
    fn get_all_devices(&self) -> Result<BlockchainState, BlockchainError>;
    fn create_device_config(&self, device: &Device) -> Result<(), BlockchainError>;
    fn delete_device(&self, dev_eui: &EUI64) -> Result<(), BlockchainError>;
    fn delete_device_session(&self, dev_addr: &[u8; 4]) -> Result<(), BlockchainError>;
    fn create_uplink(&self, packet: &[u8], answer: Option<&[u8]>, nc_id: &str) -> Result<(),BlockchainError>;
    fn join_procedure(&self, join_request: &[u8], join_accept: &[u8], nc_id: &str, dev_id: &[u8]) -> Result<bool,BlockchainError>;
    fn get_packet(&self, hash: &str) -> Result<BlockchainPacket,BlockchainError>;
    fn get_public_blockchain_state(&self) -> Result<BlockchainState, BlockchainError>;
    fn get_device_org(&self, dev_id: &[u8]) -> Result<String, BlockchainError>;
    fn get_org_anchor_address(&self, org: &str) -> Result<(IpAddr, u16), BlockchainError>;
}