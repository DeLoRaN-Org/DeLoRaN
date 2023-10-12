use std::{net::{Ipv4Addr, IpAddr}, time::SystemTime};

use lorawan::{utils::{PrettyHexSlice, eui::EUI64}, device::Device};
use reqwest::{Client, header::{HeaderMap, CONTENT_TYPE}};
use serde_json::json;

use crate::{BlockchainDeviceSession, BlockchainDeviceConfig, BlockchainState, BlockchainPacket, BlockchainError};

#[derive(Clone)]
pub struct BlockchainTCPClient {
    client: Client,
    address: Ipv4Addr,
    port: u16,
}

impl BlockchainTCPClient {
    #[deprecated(note = "Use BlockchainExeBridge instead")]
    pub fn new(address: Ipv4Addr, port: u16) -> Self {
        Self {
            client: Client::new(),
            address,
            port,
        }
    }
}


#[async_trait::async_trait]
impl crate::BlockchainClient for BlockchainTCPClient { 
    async fn get_hash(&self) -> Result<String, BlockchainError> {
        match self.client.get(format!(
            "http://{}:{}/hash",
            self.address,
            self.port,
        )).send().await {
            Ok(r) => {
                let r_text = r.text().await.unwrap();
                println!("{r_text}");
                Ok(r_text)
            }
            Err(e) => {
                eprintln!("{e}");
                Err(BlockchainError::GenericError(e.to_string()))
            }
        }
    }

    async fn get_device_session(
        &self,
        dev_addr: &[u8; 4],
    ) -> Result<BlockchainDeviceSession, BlockchainError> {
        match self.client.get(format!(
                "http://{}:{}/device/session/{}",
                self.address,
                self.port,
                PrettyHexSlice(dev_addr)
            )).send().await {
            Ok(r) => {
                let r_text = r.text().await.unwrap();
                //println!("{r_text}");
                Ok(serde_json::from_str(&r_text).unwrap())
            }
            Err(e) => {
                eprintln!("{e}");
                Err(BlockchainError::GenericError(e.to_string()))
            }
        }
    }

    async fn get_device_config(
        &self,
        dev_eui: &EUI64,
    ) -> Result<BlockchainDeviceConfig, BlockchainError> {
        match self
            .client
            .get(format!(
                "http://{}:{}/device/config/{}",
                self.address, self.port, dev_eui
            ))
            .send()
            .await
        {
            Ok(r) => {
                let r_text = r.text().await.unwrap();
                //println!("{r_text}");
                Ok(serde_json::from_str(&r_text).unwrap())
            }
            Err(e) => {
                //eprintln!("{e}");
                Err(BlockchainError::GenericError(e.to_string()))
            }
        }
    }

    async fn get_device(&self, dev_eui: &EUI64) -> Result<Device, BlockchainError> {
        match self
            .client
            .get(format!(
                "http://{}:{}/device/{}",
                self.address, self.port, dev_eui
            ))
            .send()
            .await
        {
            Ok(r) => {
                let text = r.text().await.unwrap();
                serde_json::from_str::<Device>(&text).map_err(|e| BlockchainError::GenericError(e.to_string()))
            }
            Err(e) => {
                eprintln!("{e}");
                Err(BlockchainError::GenericError(e.to_string()))
            }
        }
    }

    async fn get_all_devices(&self) -> Result<BlockchainState, BlockchainError> {
        match self
            .client
            .get(format!("http://{}:{}/allDevices", self.address, self.port))
            .send()
            .await
        {
            Ok(r) => {
                let text_r = r.text().await.unwrap();
                //println!("{text_r}");
                let state: BlockchainState = serde_json::from_str(&text_r).unwrap();
                Ok(state)
            }
            Err(e) => {
                eprintln!("{e}");
                Err(BlockchainError::GenericError(e.to_string()))
            }
        }
    }

    async fn create_device_config(&self, device: &Device) -> Result<(), BlockchainError> {
        let jbody = json!({
            "device": device,
            "n_id": 1
        });

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        match self
            .client
            .post(format!(
                "http://{}:{}/createDevice",
                self.address, self.port
            ))
            .headers(headers)
            .body(jbody.to_string())
            .send()
            .await
        {
            Ok(_r) => {
                //println!("{r:?}");
                Ok(())
            }
            Err(e) => {
                eprintln!("{e}");
                Err(BlockchainError::GenericError(e.to_string()))
            }
        }
    }

    async fn delete_device(&self, dev_eui: &EUI64) -> Result<(), BlockchainError> {
        match self
            .client
            .delete(format!(
                "http://{}:{}/device/config/{}",
                self.address, self.port, dev_eui
            ))
            .send()
            .await
        {
            Ok(_r) => {
                //println!("{}", r.text().await.unwrap());
                Ok(())
            }
            Err(e) => {
                eprintln!("{e}");
                Err(BlockchainError::GenericError(e.to_string()))
            }
        }
    }

    async fn delete_device_session(&self, dev_addr: &[u8; 4]) -> Result<(), BlockchainError> {
        match self
            .client
            .delete(format!(
                "http://{}:{}/device/session/{}",
                self.address,
                self.port,
                PrettyHexSlice(dev_addr)
            ))
            .send()
            .await
        {
            Ok(_r) => {
                //println!("{}", r.text().await.unwrap());
                Ok(())
            }
            Err(e) => {
                eprintln!("{e}");
                Err(BlockchainError::GenericError(e.to_string()))
            }
        }
    }
    
    async fn create_uplink(&self, packet: &[u8], answer: Option<&[u8]>, n_id: &str) -> Result<(),BlockchainError> {
        let mut jbody = json!({
            "packet": PrettyHexSlice(packet).to_string(),
            "n_id" : n_id,
            "date": format!("{}",SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis())
        });

        if let Some(v) = answer {
            jbody["answer"] = serde_json::Value::String(PrettyHexSlice(v).to_string())
        }

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        match self
            .client
            .post(format!(
                "http://{}:{}/uploadPacket",
                self.address, self.port
            ))
            .headers(headers)
            .body(jbody.to_string())
            .send()
            .await
        {
            Ok(_) => {
                Ok(())
            }
            Err(e) => {
                eprintln!("{e}");
                Err(BlockchainError::GenericError(e.to_string()))
            }
        }
    }
    
    async fn get_packet(&self, hash: &str) -> Result<BlockchainPacket,BlockchainError> {
        match self
            .client
            .get(format!(
                "http://{}:{}/getPacket/{}",
                self.address, self.port, hash
            ))
            .send()
            .await
        {
            Ok(r) => {
                let text = r.text().await.unwrap();
                Ok(serde_json::from_str(&text).unwrap())
            }
            Err(e) => {
                eprintln!("{e}");
                Err(BlockchainError::GenericError(e.to_string()))
            }
        }
    }

    async fn get_public_blockchain_state(&self) -> Result<BlockchainState, BlockchainError> {
        match self
            .client
            .get(format!("http://{}:{}/getPublicBlockchainState", self.address, self.port))
            .send().await {
            Ok(r) => {
                let text_r = r.text().await.unwrap();
                //println!("{text_r}");
                let state: BlockchainState = serde_json::from_str(&text_r).unwrap();
                Ok(state)
            }
            Err(e) => {
                eprintln!("{e}");
                Err(BlockchainError::GenericError(e.to_string()))
            }
        }
    }

    async fn get_device_org(&self, dev_id: &[u8]) -> Result<String, BlockchainError> {
        match self
            .client
            .get(format!(
                "http://{}:{}/device/session/{}",
                self.address,
                self.port,
                PrettyHexSlice(dev_id)
            ))
            .send()
            .await
        {
            Ok(r) => {
                let text_r = r.text().await.unwrap();
                //println!("{text_r}");
                Ok(text_r)
            }
            Err(e) => {
                eprintln!("{e}");
                Err(BlockchainError::GenericError(e.to_string()))
            }
        }
    }

    async fn get_org_anchor_address(&self, _org: &str) -> Result<(IpAddr, u16), BlockchainError> {
        todo!("look on phdind")
    }
}