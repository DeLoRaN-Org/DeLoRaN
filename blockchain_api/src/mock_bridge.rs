use rand::RngCore;
use std::{net::{IpAddr, Ipv4Addr}, vec};

use lorawan::{
    device::{
        session_context::{ApplicationSessionContext, NetworkSessionContext, SessionContext},
        Device, DeviceClass, LoRaWANVersion,
    },
    encryption::key::Key,
    utils::eui::EUI64,
};
use rand::SeedableRng;

use crate::{
    BlockchainClient, BlockchainDeviceConfig, BlockchainDeviceSession, BlockchainError,
    BlockchainPacket, BlockchainState, HyperledgerJoinDeduplicationAns,
};


/// `s!` is a macro that converts an expression into a `String`.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// let var: &str = "Hello, World!"; 
/// let my_string: String = s!(var); 
/// assert!(String::from("Hello, World!"), my_string); // Prints: Hello, World! 
/// ```
///
/// # Note
///
/// This macro is a shorthand for `String::from()`. It takes an expression and returns a `String`.
macro_rules! s {
    ($e:tt) => {
        String::from($e)
    };
}

#[derive(Default)]
pub struct BlockchainMockClient {}

impl BlockchainMockClient {
    pub fn create_uninitialized_device(t_seed: &[u8]) -> Device {
        let mut seed = [0u8; 32];
        let old_length = t_seed.len();
        for i in 0..32 {
            seed[i] = t_seed[i % old_length];
        }
    
        let mut generator = rand::rngs::StdRng::from_seed(seed);
    
        let mut key: [u8; 16] = [0_u8; 16];
        let mut s_key: [u8; 16] = [0_u8; 16];
        let mut d_eui: [u8; 8] = [0_u8; 8];
        let mut j_eui: [u8; 8] = [0_u8; 8];
    
        let mut home_net_id = [0_u8; 3];
        let mut d_addr = [0_u8; 4];
    
        generator.fill_bytes(&mut key);
    
        if t_seed.len() == 4 {
            d_addr.copy_from_slice(t_seed);
            generator.fill_bytes(&mut d_eui);
        } else {
            // == 8
            d_eui.copy_from_slice(t_seed);
            generator.fill_bytes(&mut d_addr);
        }
    
        generator.fill_bytes(&mut j_eui);
        generator.fill_bytes(&mut s_key);
        generator.fill_bytes(&mut home_net_id);
    
        let key = Key::from(key);
    
        let d_eui = EUI64::from(d_eui);
        let j_eui = EUI64::from(j_eui);
    
        Device::new(
            DeviceClass::A,
            None,
            d_eui,
            j_eui,
            key,
            key,
            LoRaWANVersion::V1_1,
        )
    }

    pub fn create_initialized_device(t_seed: &[u8]) -> Device {
        let mut seed = [0u8; 32];
        let old_length = t_seed.len();
        for i in 0..32 {
            seed[i] = t_seed[i % old_length];
        }
    
        let mut generator = rand::rngs::StdRng::from_seed(seed);
    
        let mut s_key: [u8; 16] = [0_u8; 16];
        let mut home_net_id = [0_u8; 3];
        let mut d_addr = [0_u8; 4];
        
        if t_seed.len() == 4 {
            d_addr.copy_from_slice(t_seed);
        } else { // == 8
            generator.fill_bytes(&mut d_addr);
        }
    
        generator.fill_bytes(&mut s_key);
        generator.fill_bytes(&mut home_net_id);
        let s_key = Key::from(s_key);


        let mut device = Self::create_uninitialized_device(t_seed);
    
        let network_context =
            NetworkSessionContext::new(s_key, s_key, s_key, home_net_id, d_addr, 0, 0, 0);
    
        let application_context = ApplicationSessionContext::new(
            Key::from_hex("5560CC0B0DC37BEBBFB39ACD337DD34D").unwrap(),
            0,
        );
    
        device.set_activation_abp(SessionContext::new(application_context, network_context));
        device
    }

}

#[derive(Default, Clone)]
pub struct BlockchainMockClientConfig;

impl BlockchainClient for BlockchainMockClient {
    type Config = BlockchainMockClientConfig;

    async fn from_config(_config: &Self::Config) -> Result<Box<Self>, BlockchainError> {
        Ok(Box::default())
    }

    async fn get_hash(&self) -> Result<String, BlockchainError> {
        Ok(s!(
            "6325a7e56e8d968ece6ce624dd45cf9df46c7d366f9b88cb8f737698f2bcad88"
        ))
    }

    async fn get_device_session(
        &self,
        dev_addr: &[u8; 4],
    ) -> Result<BlockchainDeviceSession, BlockchainError> {
        let d = Self::create_initialized_device(dev_addr);
        Ok(BlockchainDeviceSession::from(
            d.session().unwrap(),
            d.dev_eui(),
        ))
    }

    async fn get_device_config(
        &self,
        dev_eui: &EUI64,
    ) -> Result<BlockchainDeviceConfig, BlockchainError> {
        let d = Self::create_initialized_device(&**dev_eui);
        Ok((&d).into())
    }

    async fn get_device(&self, dev_eui: &EUI64) -> Result<Device, BlockchainError> {
        Ok(Self::create_initialized_device(&**dev_eui))
    }

    async fn get_all_devices(&self) -> Result<BlockchainState, BlockchainError> {
        Ok(BlockchainState {
            configs: vec![],
            packets: vec![],
        })
    }

    async fn create_device_config(&self, _device: &Device) -> Result<(), BlockchainError> {
        Ok(())
    }

    async fn delete_device(&self, _dev_eui: &EUI64) -> Result<(), BlockchainError> {
        Ok(())
    }

    async fn delete_device_session(&self, _dev_addr: &[u8; 4]) -> Result<(), BlockchainError> {
        Ok(())
    }

    async fn create_uplink(
        &self,
        _packet: &[u8],
        _answer: Option<&[u8]>,
    ) -> Result<(), BlockchainError> {
        Ok(())
    }

    async fn join_procedure(&self, _join_request: &[u8], _join_accept: &[u8], _dev_eui: &EUI64) -> Result<HyperledgerJoinDeduplicationAns,BlockchainError> {
    Ok(HyperledgerJoinDeduplicationAns {
        winner: "a1b2c3d4".to_owned(),
            keys: vec!["key1".to_owned(), "key2".to_owned()]
        })
    }

    

    async fn get_packet(&self, hash: &str) -> Result<BlockchainPacket, BlockchainError> {
        Ok(BlockchainPacket {
            hash: s!(hash),
            timestamp: s!("1698963554491"),
            dev_id: s!("a1b2c3d4"),
            length: 42,
            sf: 7,
            gws: vec!["gw1".to_owned(), "gw2".to_owned()],
        })
    }

    async fn get_public_blockchain_state(&self) -> Result<BlockchainState, BlockchainError> {
        self.get_all_devices().await
    }

    async fn get_device_org(&self, _dev_id: &[u8]) -> Result<String, BlockchainError> {
        Ok(s!("Org1MSP"))
    }

    async fn get_org_anchor_address(&self, _org: &str) -> Result<(IpAddr, u16), BlockchainError> {
        Ok((IpAddr::V4(Ipv4Addr::LOCALHOST), 1312))
    }
    
    async fn session_generation(&self, _keys: &[&str], _dev_eui: &str) -> Result<(),BlockchainError> {
        Ok(())
    }
}
