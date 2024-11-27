use std::fs::OpenOptions;
use std::net::IpAddr;
use std::time::{Duration, Instant};
use std::{collections::HashMap, time::SystemTime};
use lorawan::{utils::{PrettyHexSlice, eui::EUI64}, device::Device};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use tokio::process::Command;
use std::io::Write;


use crate::{BlockchainDeviceConfig, BlockchainDeviceSession, BlockchainError, BlockchainPacket, BlockchainState, HyperledgerJoinDeduplicationAns};

#[allow(non_snake_case)]
#[derive(Serialize)]
pub struct BlockchainArgs {
    Args: Vec<String>
}

#[derive(Deserialize, Debug)]
pub struct BlockchainAns<T> {
    content: Option<T>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct HyperledgerInvokeAns {
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

#[derive(Clone)]
pub struct BlockchainExeConfig {
    pub orderer_addr: String,
    pub channel_name: String,
    pub chaincode_name: String,
    pub orderer_ca_file_path: Option<String>,
}

const STATUS_OK: u16 = 200;
const STATUS_KEY: &str = "status:";
const DEFAULT_CAFILE_PATH: &str = "/opt/fabric/crypto/orderer-ca.crt";
const LOG_FILE_PATH: &str = "/root/API_invoke_times.csv";

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

    async fn create_command<T: for<'a> Deserialize<'a>>(&self, invoke: bool , args: BlockchainArgs, transient_data: Option<&HashMap<&'static str, Vec<u8>>>) -> Result<BlockchainAns<T>, String> {        
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
            "--cafile", { if let Some(v) = &self.orderer_ca_file_path { v } else { DEFAULT_CAFILE_PATH } },
            ];
        if !transient_string.is_empty() { peer_args.extend_from_slice(&["--transient", &transient_string]) }
        if invoke { peer_args.push("--waitForEvent") }
        
        //println!("peer {}", peer_args.join(" "));
        //return Err("abbalabba".to_owned());

        let before = Instant::now();
        let output = Command::new("peer").args(peer_args).output().await.map_err(|e| e.to_string())?;
        let after = Instant::now();
        
        if true {
            let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(LOG_FILE_PATH)
            .expect("Failed to open file");
            writeln!(file, "{},{}", SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis(), (after - before).as_millis()).expect("Error while logging time to file");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        //println!("stdout: {}", stdout);
        //println!("stderr: {}", stderr);

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
            let msg = ans.msg.split(STATUS_KEY).nth(1).ok_or(format!("Error parsing output, no status found. {last_stderr_line}"))?.trim();
            let mut success = false;


            if let Some(index) = msg.find(' ') {
                let (str_status, message) = msg.split_at(index);
                status = str_status.parse().unwrap();
                if status == STATUS_OK {
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
                        return serde_json::from_str::<BlockchainAns<T>>(&t_value[2..=len-2]).map_err(|e| e.to_string());       
                    }
                }                
            } else {
                status = msg.parse().unwrap();
                if status == STATUS_OK {
                    success = true;
                }
            }

            if success {
                Ok(BlockchainAns { content: None })
            } else {
                Err(format!("{STATUS_KEY} {}, {}: {}", status, key.unwrap_or(""), value.unwrap_or("")))
            }
        }
        else {
            serde_json::from_str::<BlockchainAns<T>>(stdout.trim()).or_else(|_| {
                let a = serde_json::from_str::<Value>(stdout.trim()).unwrap();
                Ok(BlockchainAns {
                    content: serde_json::from_str::<T>(a.get("content").unwrap().as_str().unwrap()).ok(),
                })
            })
            //FIXME non era cosi non so che succede, controllare il metodo di creazione dei device
            //serde_json::from_str::<BlockchainAns<T>>(stdout.trim()).map_err(|e| e.to_string()) 
        }
    }
}


//TODO Remove, just for convergence times
impl BlockchainExeClient {
    pub async fn create_flag(&self) -> Result<(), BlockchainError> {
        let args = BlockchainArgs {
            Args: vec![
                "CreateFlag".to_owned(),
            ],
        };

        self.create_command::<()>(true,args, None).await.map_err(BlockchainError::GenericError)?;
        Ok(())
    }
    
    pub async fn clear_flag(&self) -> Result<(), BlockchainError> {
        let args = BlockchainArgs {
            Args: vec![
                "ClearFlag".to_owned(),
            ],
        };

        self.create_command::<()>(true,args, None).await.map_err(BlockchainError::GenericError)?;
        Ok(())
    }

    pub async fn get_flag(&self) -> Result<String,BlockchainError> {
        let args = BlockchainArgs {
            Args: vec![
                "ReadFlag".to_owned(),
            ],
        };

        self.create_command(false,args, None).await
        .map_err(BlockchainError::GenericError)?
        .content.ok_or(BlockchainError::MissingContent)
    }
}

impl crate::BlockchainClient for BlockchainExeClient {
    type Config = BlockchainExeConfig;

    async fn from_config(config: &Self::Config) -> Result<Box<Self>, BlockchainError> {
        Ok(Box::new(Self::new(config.orderer_addr.clone(), config.channel_name.clone(), config.chaincode_name.clone(), config.orderer_ca_file_path.clone())))
    }

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
        let str_dev_eui = dev_eui.to_string();
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
    
    async fn create_uplink(&self, packet: &[u8], answer: Option<&[u8]>) -> Result<(),BlockchainError> {
        let date = format!("{}",SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis());
        let mut transient_data = HashMap::from([
            ("packet", packet.to_vec()),
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

        self.create_command::<()>(true,args, Some(&transient_data)).await.map_err(BlockchainError::GenericError)?;
        Ok(())
    }

    async fn join_procedure(&self, join_request: &[u8], join_accept: &[u8], dev_eui: &EUI64) -> Result<HyperledgerJoinDeduplicationAns,BlockchainError> {
        let date = format!("{}",SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis());
        let transient_data = HashMap::from([
            ("join_request", join_request.to_vec()),
            ("join_accept", join_accept.to_vec()),
            ("date", date.as_bytes().to_vec()),
        ]);
        
        let args = BlockchainArgs {
            Args: vec![
                "LoRaWANPackets:JoinRequestPreDeduplication".to_owned(),
            ],
        };
        
        self.create_command::<()>(true,args, Some(&transient_data)).await.map_err(BlockchainError::GenericError)?;
        tokio::time::sleep(Duration::from_millis(500)).await;
        

        let d_eui = dev_eui.to_string().as_bytes().to_vec();

        let transient_data = HashMap::from([
            ("dev_eui", d_eui),
        ]);
        
        let args = BlockchainArgs {
            Args: vec![
                "LoRaWANPackets:JoinRequestDeduplication".to_owned(),
            ],
        };
        let res = self.create_command::<HyperledgerJoinDeduplicationAns>(false,args, Some(&transient_data)).await.map_err(BlockchainError::GenericError)?;

        //let d_eui = dev_eui.to_string().as_bytes().to_vec();
        match res.content {
            Some(ans) => {
                Ok(ans)
                //if ans.winner != nc_id {
                //    return Ok(false)   
                //}
                //MOVED TO SESSIONGENERATION
                //let transient_data = HashMap::from([
                //    ("keys", serde_json::to_vec(&ans.keys).unwrap()),
                //    //("nc_id", nc_id.as_bytes().to_vec()),
                //    ("dev_eui", d_eui)
                //]);
                //
                //let args = BlockchainArgs {
                //    Args: vec![
                //        "LoRaWANPackets:JoinRequestSessionGeneration".to_owned(),
                //    ],
                //};
                //
                //self.create_command::<()>(true,args, Some(&transient_data)).await.map_err(BlockchainError::GenericError)?;
            },
            None => Err(BlockchainError::MissingContent)
        }
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
    
    async fn session_generation(&self, keys: &[&str], dev_eui: &str) -> Result<(),BlockchainError> {
        let transient_data = HashMap::from([
            ("keys", serde_json::to_vec(&keys).unwrap()),
            //("nc_id", nc_id.as_bytes().to_vec()),
            ("dev_eui", dev_eui.as_bytes().to_vec())
        ]);
        
        let args = BlockchainArgs {
            Args: vec![
                "LoRaWANPackets:JoinRequestSessionGeneration".to_owned(),
            ],
        };

        self.create_command::<()>(true,args, Some(&transient_data)).await.map_err(BlockchainError::GenericError)?;
        Ok(())
    }
}