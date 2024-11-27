use std::{fs::{File, OpenOptions}, net::IpAddr, time::{Instant, SystemTime, UNIX_EPOCH}};

use lorawan::{device::Device, utils::{eui::EUI64, PrettyHexSlice}};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{net::UdpSocket, sync::Mutex};
use std::io::Write;

use crate::{BlockchainDeviceConfig, BlockchainDeviceSession, BlockchainError, BlockchainPacket, BlockchainState, HyperledgerJoinDeduplicationAns};


pub struct Logger {
    log_file: Mutex<File>,
    active_logger: bool,
    logger_println: bool,
}

impl Logger {
    pub fn new(path: &str, active_logger: bool, logger_println: bool) -> Self {
        let file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .expect("Failed to open file");
        Self {
            log_file: Mutex::new(file),
            active_logger,
            logger_println
        }
    }

    pub fn now() -> u128 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
    }

    pub async fn write(&self, content: &str) {
        if self.active_logger {
            writeln!(self.log_file.lock().await, "{}", content).expect("Error while logging to file");
        }
        if self.logger_println {
            println!("{}", content)
        }
    }
    
    pub fn write_sync(&self, content: &str) {
        if self.active_logger {
            writeln!(self.log_file.blocking_lock(), "{}", content).expect("Error while logging to file");
        }
        if self.logger_println {
            println!("{}", content)
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct BlockchainUDPAns<T> {
    ok: bool,
    content: Option<T>,
    error_message: Option<String>,
}

pub struct BlockchainUDPClient {
    port: u16,
    logger: Logger,
}

impl BlockchainUDPClient {
    pub fn new(port: u16) -> Self {
        Self { port, logger: Logger::new(LOG_FILE_PATH, true, false) }
    }
}

#[derive(Clone)]
pub struct BlockchainUDPConfig {
    pub port: u16,
}

const LOG_FILE_PATH: &str = "/root/API_invoke_times.csv";

impl crate::BlockchainClient for BlockchainUDPClient {
    type Config = BlockchainUDPConfig;

    async fn from_config(config: &Self::Config) -> Result<Box<Self>, BlockchainError> {
        Ok(Box::new(Self::new(config.port)))
    }

    async fn get_device_session(&self, dev_addr: &[u8; 4]) -> Result<BlockchainDeviceSession, BlockchainError> {
        let sock = UdpSocket::bind("127.0.0.1:0").await.map_err(|_| BlockchainError::Error("Cannot connect to "))?;

        let v = json!({
            "type": "get_device_session",
            "dev_addr": PrettyHexSlice(dev_addr).to_string(),
        });

        sock.send_to(v.to_string().as_bytes(), format!("127.0.0.1:{}", self.port)).await.map_err(|_| BlockchainError::Error("Cannot send data to API server"))?;

        let mut vec = [0_u8; 1024];

        let before = Instant::now();
        let recvd = sock.recv(&mut vec).await.map_err(|_| BlockchainError::Error("Cannot receive data from API server"))?;
        let after = Instant::now();
        


        if false { self.logger.write(&format!("{},{}", Logger::now(), (after - before).as_millis())).await; }

        let BlockchainUDPAns {ok, content, error_message} = serde_json::from_str::<BlockchainUDPAns<BlockchainDeviceSession>>(std::str::from_utf8(&vec[..recvd]).unwrap()).map_err(|e| {
            println!("{e}");
            BlockchainError::JSONParsingError
        })?;
        
        if !ok {
            Err(BlockchainError::GenericError(error_message.unwrap()))
        } else {
            Ok(content.unwrap())
        }
    }

    async fn get_device_config(&self, dev_eui: &EUI64,) -> Result<BlockchainDeviceConfig, BlockchainError> {
        let sock = UdpSocket::bind("127.0.0.1:0").await.map_err(|_| BlockchainError::Error("Cannot connect to "))?;

        let v = json!({
            "type": "get_device_config",
            "dev_eui": dev_eui.to_string(),        
        });

        sock.send_to(v.to_string().as_bytes(), format!("127.0.0.1:{}", self.port)).await.map_err(|_| BlockchainError::Error("Cannot send data to API server"))?;

        let mut vec = [0_u8; 1024];

        let before = Instant::now();
        let recvd = sock.recv(&mut vec).await.map_err(|_| BlockchainError::Error("Cannot receive data from API server"))?;
        let after = Instant::now();
        
        if false { self.logger.write(&format!("{},{}", Logger::now(), (after - before).as_millis())).await; }


        let BlockchainUDPAns {ok, content, error_message} = serde_json::from_str::<BlockchainUDPAns<BlockchainDeviceConfig>>(std::str::from_utf8(&vec[..recvd]).unwrap()).map_err(|e| {
            println!("{e}");
            BlockchainError::JSONParsingError
        })?;
        
        if !ok {
            Err(BlockchainError::GenericError(error_message.unwrap()))
        } else {
            Ok(content.unwrap())
        }
    }

    async fn create_uplink(&self, packet: &[u8], answer: Option<&[u8]>) -> Result<(),BlockchainError> {
        let sock = UdpSocket::bind("127.0.0.1:0").await.map_err(|_| BlockchainError::Error("Cannot connect to "))?;

        let v = json!({
            "type": "create_uplink",
            "packet": packet,
            "answer": answer        
        });

        sock.send_to(v.to_string().as_bytes(), format!("127.0.0.1:{}", self.port)).await.map_err(|_| BlockchainError::Error("Cannot send data to API server"))?;

        let mut vec = [0_u8; 1024];

        let before = Instant::now();
        let recvd = sock.recv(&mut vec).await.map_err(|_| BlockchainError::Error("Cannot receive data from API server"))?;
        let after = Instant::now();
        
        if true { self.logger.write(&format!("{},{}", Logger::now(), (after - before).as_millis())).await; }


        let ans = serde_json::from_str::<BlockchainUDPAns<()>>(std::str::from_utf8(&vec[..recvd]).unwrap()).map_err(|e| {
            println!("{e}");
            BlockchainError::JSONParsingError
        })?;
        
        if !ans.ok {
            Err(BlockchainError::GenericError(ans.error_message.unwrap()))
        } else {
            Ok(())
        }
    }

    async fn join_procedure(&self, join_request: &[u8], join_accept: &[u8], dev_eui: &EUI64) -> Result<HyperledgerJoinDeduplicationAns,BlockchainError> {
        let sock = UdpSocket::bind("127.0.0.1:0").await.map_err(|_| BlockchainError::Error("Cannot connect to "))?;

        let v = json!({
            "type": "join_procedure",
            "join_request": join_request,
            "join_accept": join_accept,
            "dev_id": dev_eui.to_string()     
        });

        sock.send_to(v.to_string().as_bytes(), format!("127.0.0.1:{}", self.port)).await.map_err(|_| BlockchainError::Error("Cannot send data to API server"))?;

        let mut vec = [0_u8; 1024];

        let before = Instant::now();
        let recvd = sock.recv(&mut vec).await.map_err(|_| BlockchainError::Error("Cannot receive data from API server"))?;
        let after = Instant::now();
        
        if true { self.logger.write(&format!("{},{}", Logger::now(), (after - before).as_millis())).await; }

        let ans = serde_json::from_str::<BlockchainUDPAns<HyperledgerJoinDeduplicationAns>>(std::str::from_utf8(&vec[..recvd]).unwrap()).map_err(|e| {
            println!("{e}");
            BlockchainError::JSONParsingError
        })?;
        
        if !ans.ok {
            Err(BlockchainError::GenericError(ans.error_message.unwrap()))
        } else {
            Ok(ans.content.unwrap())
        }
    }

    async fn session_generation(&self, keys: &[&str], dev_eui: &str) -> Result<(),BlockchainError> {
        let sock = UdpSocket::bind("127.0.0.1:0").await.map_err(|_| BlockchainError::Error("Cannot connect to "))?;

        let v = json!({
            "type": "session_generation",
            "keys": keys,
            "dev_eui": dev_eui,
        });

        sock.send_to(v.to_string().as_bytes(), format!("127.0.0.1:{}", self.port)).await.map_err(|_| BlockchainError::Error("Cannot send data to API server"))?;

        let mut vec = [0_u8; 1024];

        let before = Instant::now();
        let recvd = sock.recv(&mut vec).await.map_err(|_| BlockchainError::Error("Cannot receive data from API server"))?;
        let after = Instant::now();
        
        if true { self.logger.write(&format!("{},{}", Logger::now(), (after - before).as_millis())).await; }

        let ans = serde_json::from_str::<BlockchainUDPAns<()>>(std::str::from_utf8(&vec[..recvd]).unwrap()).map_err(|e| {
            println!("{e}");
            BlockchainError::JSONParsingError
        })?;
        
        if !ans.ok {
            Err(BlockchainError::GenericError(ans.error_message.unwrap()))
        } else {
            Ok(())
        }
    }
    
    
    
    async fn get_hash(&self) -> Result<String, BlockchainError> {
        unimplemented!("get_hash")
    }
    async fn get_device(&self, _dev_eui: &EUI64) -> Result<Device, BlockchainError> {
        unimplemented!("get_device")
    }
    async fn get_all_devices(&self) -> Result<BlockchainState, BlockchainError> {
        unimplemented!("get_all_devices")
    }
    async fn create_device_config(&self, device: &Device) -> Result<(), BlockchainError> {
        let sock = UdpSocket::bind("127.0.0.1:0").await.map_err(|_| BlockchainError::Error("Cannot connect to "))?;

        let config: BlockchainDeviceConfig = device.into();
        let v = json!({
            "type": "create_device_config",
            "device": config,
        });

        sock.send_to(v.to_string().as_bytes(), format!("127.0.0.1:{}", self.port)).await.map_err(|_| BlockchainError::Error("Cannot send data to API server"))?;

        let mut vec = [0_u8; 1024];

        let before = Instant::now();
        let recvd = sock.recv(&mut vec).await.map_err(|_| BlockchainError::Error("Cannot receive data from API server"))?;
        let after = Instant::now();
        
        if true {
            let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(LOG_FILE_PATH)
            .expect("Failed to open file");
            writeln!(file, "{},{}", SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis(), (after - before).as_millis()).expect("Error while logging time to file");
        }

        let ans = serde_json::from_str::<BlockchainUDPAns<()>>(std::str::from_utf8(&vec[..recvd]).unwrap()).map_err(|e| {
            println!("{e}");
            BlockchainError::JSONParsingError
        })?;
        
        if !ans.ok {
            Err(BlockchainError::GenericError(ans.error_message.unwrap()))
        } else {
            Ok(())
        }
    }
    async fn delete_device(&self, _dev_eui: &EUI64) -> Result<(), BlockchainError> {
        unimplemented!("delete_device")
    }
    async fn delete_device_session(&self, _dev_addr: &[u8; 4]) -> Result<(), BlockchainError> {
        unimplemented!("delete_device_session")
    }
    async fn get_packet(&self, _hash: &str) -> Result<BlockchainPacket,BlockchainError> {
        unimplemented!("get_packet")
    }
    async fn get_public_blockchain_state(&self) -> Result<BlockchainState, BlockchainError> {
        unimplemented!("get_public_blockchain_state")
    }
    async fn get_device_org(&self, _dev_id: &[u8]) -> Result<String, BlockchainError> {
        unimplemented!("get_device_org")
    }
    async fn get_org_anchor_address(&self, _org: &str) -> Result<(IpAddr, u16), BlockchainError> {
        unimplemented!("get_org_anchor_address")
    }
}