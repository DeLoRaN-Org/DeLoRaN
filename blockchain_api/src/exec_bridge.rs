#![allow(non_snake_case)]
use std::io::Write;
use std::net::IpAddr;
use std::{collections::HashMap, time::{SystemTime, Instant}, fs::OpenOptions};
use lorawan::{utils::{PrettyHexSlice, eui::EUI64}, device::Device};
use serde::{Serialize, Deserialize};
use tokio::process::Command;


use crate::{BlockchainDeviceSession, BlockchainDeviceConfig, BlockchainState, BlockchainPacket, BlockchainError};

#[derive(Serialize)]
struct BlockchainArgs {
    Args: Vec<String>
}

#[derive(Deserialize)]
struct BlockchainAns<T> {
    content: Option<T>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct HyperledgerInvokeAns {
    level: String,
    ts: f32,
    name: String,
    caller: String,
    msg: String
}

trait TakeLastLine { //for fun
    fn take_last_line(&self) -> Option<String>;
}

impl TakeLastLine for std::borrow::Cow<'_, str> {
    fn take_last_line(&self) -> Option<String> {
        self.trim().rfind('\n').map(|index| self[index + 1..].trim().to_string())
    }
}

#[derive(Clone, Debug)]
pub struct BlockchainExeClient {
    orderer_addr: String,
    channel_name: String,
    chaincode_name: String,
    orderer_ca_file_path: Option<String>,
}



impl BlockchainExeClient {
    pub fn new<I>(addr: I, channel_name: I, chaincode_name: I, orderer_ca_file_path: Option<I>) -> BlockchainExeClient
    where 
        I: Into<String>,
    {

        Self {
            orderer_addr: addr.into(),
            channel_name: channel_name.into(),
            chaincode_name: chaincode_name.into(),
            orderer_ca_file_path: orderer_ca_file_path.map(|v| v.into()),
        }
    }

    async fn create_command<T: for<'a> Deserialize<'a>>(&self, invoke: bool , args: BlockchainArgs, transient_data: Option<HashMap<&'static str, Vec<u8>>>) -> Result<BlockchainAns<T>, String> {        
        let transient_string = if let Some(v) = transient_data { serde_json::to_string(&v).unwrap() } else { String::new() };
        
        let args_string = serde_json::to_string(&args).unwrap();

        let mut peer_args = vec![
            "chaincode",
            { if invoke { "invoke" } else { "query" }},
            "-o", &self.orderer_addr,
            "-C", &self.channel_name, 
            "-n", &self.chaincode_name,
            "-c", args_string.trim(),
            "--tls",
            "--cafile", { if let Some(v) = &self.orderer_ca_file_path { v } else {"/opt/fabric/crypto/orderer-ca.crt"} },
            ];
        if !transient_string.is_empty() { peer_args.extend_from_slice(&["--transient", &transient_string]) }
        if invoke { peer_args.push("--waitForEvent") }
        
        //println!("peer {}", peer_args.join(" "));
        //return Err("abbalabba".to_owned());

        //let before = Instant::now();
        let output = Command::new("peer").args(peer_args).output().await.map_err(|e| e.to_string())?;
        //let after = Instant::now();
        
        //{
        //    let mut file = OpenOptions::new()
        //    .append(true)
        //    .create(true)
        //    .open("/root/API_invoke_times.csv")
        //    .expect("Failed to open file");
        //    writeln!(file, "{},{}", SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis(), (after - before).as_millis()).expect("Error while logging time to file");
        //}

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        if invoke || stdout.is_empty() && !stderr.is_empty() {
            let last_stderr_line = stderr.take_last_line().ok_or("Stderr empty".to_owned())?;
            let ans = serde_json::from_str::<HyperledgerInvokeAns>(&last_stderr_line).or::<String>(Ok(HyperledgerInvokeAns {
                level: "".to_owned(),
                ts: 0.0,
                name: "".to_owned(),
                caller: "".to_owned(),
                msg: last_stderr_line.clone(),
            }))?;

            let mut key: Option<&str> = None;
            let mut value: Option<&str> = None;
            let status: u16;
            let msg = ans.msg.split("status:").nth(1).ok_or(format!("Error parsing output, no status found. {last_stderr_line}"))?.trim();
            let mut success = false;


            if let Some(index) = msg.find(' ') {
                let (str_status, message) = msg.split_at(index);
                status = str_status.parse().unwrap();
                if status == 200 {
                    success = true;
                }
                
                if !message.trim().is_empty() {
                    let (mut t_key,mut t_value) = message.find(':').map(|index| message.split_at(index)).ok_or("Malformed output message".to_owned())?;
                    t_key = t_key.trim();
                    t_value = t_value.trim();
                    
                    //println!("#{t_key}# #{t_value}#");
                    let len = t_value.len();
                    if t_key == "message" {
                        key = Some(t_key);
                        value = Some(&t_value[2..=len-2]);
                    } else {
                        //println!("{}", &t_value[2..=len-2]);
                        return serde_json::from_str::<BlockchainAns<T>>(&t_value[2..=len-2]).map_err(|e| e.to_string());       
                    }
                }                
            } else {
                status = msg.parse().unwrap();
                if status == 200 {
                    success = true;
                }
            }

            if success {
                Ok(BlockchainAns { content: None })
            } else {
                Err(format!("status: {}, {}: {}", status, key.unwrap_or(""), value.unwrap_or("")))
            }
        }
        else { serde_json::from_str::<BlockchainAns<T>>(stdout.trim()).map_err(|e| e.to_string()) }
    }
}

#[async_trait::async_trait]
impl crate::BlockchainClient for BlockchainExeClient {
    
    async fn get_hash(&self) -> Result<String, BlockchainError> {
        let args = BlockchainArgs {
            Args: vec![
                "GetChainHash".to_owned()
            ],
        };

        let ans = self.create_command(false,args, None).await.map_err(BlockchainError::GenericError)?;
        ans.content.ok_or(BlockchainError::MissingContent)
    }

    async fn get_device_session(
        &self,
        dev_addr: &[u8; 4],
    ) -> Result<BlockchainDeviceSession, BlockchainError> {
        let str_dev_addr = PrettyHexSlice(dev_addr).to_string();
        let args = BlockchainArgs {
            Args: vec![
                "ReadDeviceSession".to_owned(),
                str_dev_addr
            ],
        };

        let ans = self.create_command(false,args, None).await.map_err(BlockchainError::GenericError)?;
        ans.content.ok_or(BlockchainError::MissingContent)
    }

    async fn get_device_config(
        &self,
        dev_eui: &EUI64,
    ) -> Result<BlockchainDeviceConfig, BlockchainError> {
        let str_dev_eui = PrettyHexSlice(&**dev_eui).to_string();
        let args = BlockchainArgs {
            Args: vec![
                "ReadDeviceConfig".to_owned(),
                str_dev_eui
            ],
        };

        let ans = self.create_command(false,args, None).await.map_err(BlockchainError::GenericError)?;
        ans.content.ok_or(BlockchainError::MissingContent)
    }

    async fn get_device(&self, dev_eui: &EUI64) -> Result<Device, BlockchainError> {
        let str_dev_eui = PrettyHexSlice(&**dev_eui).to_string();
        let args = BlockchainArgs {
            Args: vec![
                "ReadDevice".to_owned(),
                str_dev_eui
            ],
        };

        let ans = self.create_command(false,args, None).await.map_err(BlockchainError::GenericError)?;
        ans.content.ok_or(BlockchainError::MissingContent)
    }

    async fn get_all_devices(&self) -> Result<BlockchainState, BlockchainError> {
        let args = BlockchainArgs {
            Args: vec![
                "GetAllDevices".to_owned(),
            ],
        };

        let ans = self.create_command(false,args, None).await.map_err(BlockchainError::GenericError)?;
        ans.content.ok_or(BlockchainError::MissingContent)
    }

    async fn create_device_config(&self, device: &Device) -> Result<(), BlockchainError> {
        let config: BlockchainDeviceConfig = device.into();
        let str_device = serde_json::to_string(&config).unwrap();
        let args = BlockchainArgs {
            Args: vec![
                "CreateDeviceConfig".to_owned(),
                str_device
            ],
        };

        self.create_command::<()>(true,args, None).await.map_err(BlockchainError::GenericError)?;
        Ok(())
    }

    async fn delete_device(&self, dev_eui: &EUI64) -> Result<(), BlockchainError> {
        let str_dev_eui = PrettyHexSlice(&**dev_eui).to_string();
        let args = BlockchainArgs {
            Args: vec![
                "DeleteDevice".to_owned(),
                str_dev_eui
            ],
        };

        self.create_command::<()>(true,args, None).await.map_err(BlockchainError::GenericError)?;
        Ok(())
    }

    async fn delete_device_session(&self, dev_addr: &[u8; 4]) -> Result<(), BlockchainError> {
        let str_dev_addr = PrettyHexSlice(dev_addr).to_string();
        let args = BlockchainArgs {
            Args: vec![
                "DeleteDeviceSession".to_owned(),
            str_dev_addr
            ],
        };

        self.create_command::<()>(true,args, None).await.map_err(BlockchainError::GenericError)?;
        Ok(())
    }
    
    async fn create_uplink(&self, packet: &[u8], answer: Option<&[u8]>, n_id: &str) -> Result<(),BlockchainError> {
        let date = format!("{}",SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis());
        let mut transient_data = HashMap::from([
            ("packet", packet.to_vec()),
            ("n_id", n_id.as_bytes().to_vec()),
            ("date", date.as_bytes().to_vec()),
        ]);
        if let Some(b) = answer {
            transient_data.insert("answer", b.to_vec());
        }
        
        let args = BlockchainArgs {
            Args: vec![
                "LoRaWANPackets:CreatePacket".to_owned(),
            ],
        };

        self.create_command::<()>(true,args, Some(transient_data)).await.map_err(BlockchainError::GenericError)?;
        Ok(())
    }
    
    async fn get_packet(&self, hash: &str) -> Result<BlockchainPacket,BlockchainError> {
        let args = BlockchainArgs {
            Args: vec![
                "LoRaWANPackets:ReadPacket".to_owned(),
                hash.to_owned()
            ],
        };

        let ans = self.create_command(false,args, None).await.map_err(BlockchainError::GenericError)?;
        ans.content.ok_or(BlockchainError::MissingContent)

    }

    async fn get_public_blockchain_state(&self) -> Result<BlockchainState, BlockchainError> {
        let args = BlockchainArgs {
            Args: vec![
                "LoRaWANPackets:GetPublicBlockchainState".to_owned(),
            ],
        };

        let ans = self.create_command(false,args, None).await.map_err(BlockchainError::GenericError)?;
        ans.content.ok_or(BlockchainError::MissingContent)

    }
    
    async fn get_device_org(&self, dev_id: &[u8]) -> Result<String, BlockchainError> {
        let dev_id_str = PrettyHexSlice(dev_id).to_string();
        let args = BlockchainArgs {
            Args: vec![
                "GetDeviceOrg".to_owned(),
                dev_id_str,
            ],
        };

        let ans = self.create_command(false,args, None).await.map_err(BlockchainError::GenericError)?;
        ans.content.ok_or(BlockchainError::MissingContent)
    }

    async fn get_org_anchor_address(&self, _org: &str) -> Result<(IpAddr, u16), BlockchainError> {
        todo!("look on phdind")
    }
}