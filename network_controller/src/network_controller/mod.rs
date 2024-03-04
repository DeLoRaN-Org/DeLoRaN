use std::sync::Arc;

use blockchain_api::{BlockchainError, BlockchainClient};
use lorawan::{utils::{increment_nonce, nonce_valid, PrettyHexSlice, traits::ToBytesWithContext, errors::LoRaWANError, eui::EUI64}, device::Device, lorawan_packet::{LoRaWANPacket, join::{JoinAcceptPayload, JoinRequestType}, payload::Payload, mhdr::{MHDR, MType, Major}, mac_payload::MACPayload, fhdr::FHDR, fctrl::{FCtrl, DownlinkFCtrl}, mac_commands}};
use lorawan_device::{communicator::LoRaWANCommunicator, configs::UDPNCConfig, devices::debug_device::DebugCommunicator};
use openssl::sha::sha256;
use serde::{Serialize, Deserialize};

use tokio::{net::UdpSocket, task::JoinHandle};
use crate::utils::error::NCError;

#[derive(Clone)]
pub struct NetworkController {
    pub n_id: &'static str,
}

//#[derive(Clone, Serialize, Deserialize)]
//#[deprecated(note="Use NetworkControllerUDPConfig instead")]
//pub struct NetworkControllerTCPConfig {
//    pub tcp_dev_port: u16,
//    pub tcp_nc_port: u16,
//}

#[derive(Clone, Serialize, Deserialize)]
pub struct NetworkControllerUDPConfig {
    pub udp_dev_port: u16,
    pub udp_nc_port: u16,
}

impl NetworkController {
    pub fn new(n_id: &'static str) -> Self {
        Self {
            n_id,
        }
    }

    async fn handle_join_request(join_request: &[u8], bc_client: &Arc<impl BlockchainClient>) -> Result<(Vec<u8>, EUI64), NCError> {
        let packet = LoRaWANPacket::from_bytes(join_request, None, true)?;
        if let Payload::JoinRequest(jr_p) = packet.payload() {
            match bc_client.get_device_config(jr_p.dev_eui()).await  {
                Err(e) => {
                    eprintln!("{e:?}");
                    Err(NCError::BlockchainError(BlockchainError::GenericError(e.to_string())))
                },
                Ok(device_config) => {
                    let mut device: Device = device_config.into();
                    let device_nonce_u16 = device.dev_nonce() as u16;
                    let (dev_nonce_valid, dev_nonce_looped) = nonce_valid(jr_p.dev_nonce(), device_nonce_u16);
                    if !dev_nonce_valid { Err(NCError::InvalidJoinRequest(format!("Invalid dev_nonce, expected > {device_nonce_u16}, received {}", jr_p.dev_nonce()))) }
                    else {
                        LoRaWANPacket::validate_mic(join_request, &packet, &device)?;
                        
                        //let dev_addr = [0_u8; 4].map(|_| rand::random::<u8>()); 
                        let mut dl_settings = 0_u8;
                        let opt_neg_v1_1 = 0b10000000;
                        dl_settings |= 0b00010001; //TODO per ora rx1_dr_offset 1, rx2_data_rate 1, capire come farle bene poi
                        if device.version().is_1_1_or_greater() {
                            dl_settings |= opt_neg_v1_1
                        }

                        let mut dev_addr = [0_u8; 4]; 
                        let dev_addr_sha256 = sha256(&[device.dev_eui().as_slice(), device.join_eui().as_slice(), &jr_p.dev_nonce().to_be_bytes()].concat());
                        dev_addr.copy_from_slice(&dev_addr_sha256[..4]);


                        let join_accept = JoinAcceptPayload::new(
                            JoinRequestType::JoinRequest, 
                            device.join_context_mut().join_nonce_autoinc(), 
                            [1,2,3], 
                            dev_addr, 
                            dl_settings, 
                            2, 
                            None
                        );
                        
                        device.set_dev_nonce(increment_nonce(jr_p.dev_nonce(), device.dev_nonce(), dev_nonce_looped));
                        device.generate_session_context(&join_accept)?;
                        device.set_last_join_request_received(JoinRequestType::JoinRequest);

                        let packet = LoRaWANPacket::new(MHDR::new(MType::JoinAccept, Major::R1), Payload::JoinAccept(join_accept));
                        //println!("{device}");
                        
                        let join_accept = packet.to_bytes_with_context(&device).map_err(NCError::from)?;
                        
                        //let session = BlockchainDeviceSession::from(device.session().unwrap(), device.dev_eui());
                        //match bc_client.create_join(&join_request, &join_accept, n_id).await {
                        //    Ok(_) =>  {
                        //        println!("Created device session successfully for device {}", PrettyHexSlice(&session.dev_addr));
                        //    },
                        //    Err(_) => println!("Error updating counter"),
                        //} //TODO tempo di risposta ottimo, upload della join dopo  ma chissà se si rischia di rompere qualcosa così
                        Ok((join_accept, *device.dev_eui()))
                    }
                }
            }
        } else { Err(NCError::InvalidJoinRequest("Not a join request".to_string())) }
    }

    async fn handle_unconfirmed_data_up(data_up: &[u8], bc_client: &Arc<impl BlockchainClient>) -> Result<Device, NCError> {
        let packet = LoRaWANPacket::from_bytes(data_up, None, true)?;
        if let Payload::MACPayload(payload) = packet.into_payload() {
            let dev_addr = payload.fhdr().dev_addr();

            if let Ok(session) = bc_client.get_device_session(&dev_addr).await {
                let fcnt_u16 = payload.fhdr().fcnt();
                let current_fcnt = session.f_cnt_up;  
                        
                let (fcnt_up_valid, _fcnt_up_looped) = nonce_valid(fcnt_u16, current_fcnt as u16);
                if !fcnt_up_valid { return Err(NCError::InvalidUplink(format!("Invalid fcnt_up, expected > {current_fcnt}, received {fcnt_u16}"))); }
                
                //let new_fcnt_up = increment_nonce(fcnt_u16, current_fcnt, fcnt_up_looped);
                //device.session_mut().unwrap().network_context_mut().update_f_cnt_up(new_fcnt_up);

                let device = session.into();
                let p = LoRaWANPacket::from_bytes(data_up, Some(&device), true)?; 

                if let Payload::MACPayload(_mp) = p.payload() {
                    //println!("{:?}", String::from_utf8_lossy(mp.frm_payload().unwrap()));
                    if payload.is_application() {
                        //let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                        //let data_to_send = json!({
                        //    "payload": (mp.frm_payload().unwrap_or(&Vec::new())),
                        //    "dev_addr": dev_addr,
                        //    "tmst": now.as_secs()
                        //});
                        //utils::uplink_to_application_server(data_to_send.to_string().as_bytes()).await?;
                    } else {
                        let mac_commands = mac_commands::EDMacCommands::from_bytes(&p.payload().to_bytes_with_context(&device).unwrap())?;
                        println!("{mac_commands:?}");
                        //TODO analyze mac commands and act accordingly
                    }
                }
                Ok(device)
            } else { Err(NCError::UnknownDevAddr(payload.fhdr().dev_addr())) }
        } else { Err(NCError::InvalidUplink("Not a MACPayload payload".to_string())) }
    }

    async fn handle_confirmed_data_up(data_up: &[u8], bc_client: &Arc<impl BlockchainClient>) -> Result<(Vec<u8>, EUI64), NCError> {
        let mut device = Self::handle_unconfirmed_data_up(data_up, bc_client).await?;
        
        let dev_addr = *device.session().unwrap().network_context().dev_addr();
        let fctrl = FCtrl::Downlink(DownlinkFCtrl::new(false, false, true, false, 0));
        let mut fhdr = FHDR::new(dev_addr, fctrl);
        
        let fport = Some(1);
        let session_context = device.session_mut().ok_or(LoRaWANError::SessionContextMissing)?;
        let new_value = if fport.is_none() || fport == Some(0) {
            session_context.network_context_mut().nf_cnt_dwn_autoinc() as u16
        } else {
            session_context.application_context_mut().af_cnt_dwn_autoinc() as u16
        };
        fhdr.set_fcnt(new_value);

        let downlink_payload = Payload::MACPayload(MACPayload::new(fhdr, fport, Some("Confirmed Uplink answer".bytes().collect())));
        let mhdr = MHDR::new(MType::UnconfirmedDataDown, Major::R1);
        let packet = LoRaWANPacket::new(mhdr, downlink_payload);

        let data_down = packet.to_bytes_with_context(&device).map_err(NCError::from)?;
        Ok((data_down, *device.dev_eui()))
    }
    
    async fn dispatch_task(mhdr: &MHDR, buf: &[u8], bc_client: &Arc<impl BlockchainClient>) -> Result<Option<(Vec<u8>, EUI64)>, NCError> {
        match mhdr.mtype() {
            MType::JoinRequest => {
                Ok(Some(Self::handle_join_request(buf, bc_client).await?))
            },
            MType::UnconfirmedDataUp => {
                Self::handle_unconfirmed_data_up(buf, bc_client).await?;
                Ok(None)
            },
            MType::ConfirmedDataUp => {
                Ok(Some(Self::handle_confirmed_data_up(buf, bc_client).await?))
            },
            MType::RejoinRequest => {
                unimplemented!("RejoinRequest")
            },
            
            MType::UnconfirmedDataDown |
            MType::ConfirmedDataDown |
            MType::JoinAccept => {
                eprintln!("received {mhdr:?}, ignoring");
                Err(NCError::InvalidUplink("Received downlink".to_string()))
            } //TODO -> ignore?
            MType::Proprietary => {
                unimplemented!("Proprietary")
            }
        }
    }

    async fn communicator_routine<LC,BC>(config: &'static LC::Config, n_id: &'static str, blockchain_config: &BC::Config) 
    where LC: LoRaWANCommunicator + Sync + Send + 'static, 
          BC: BlockchainClient + 'static { //TODO too many statics? maybe not

        let client: Arc<BC> = Arc::new(*BC::from_config(blockchain_config).await.unwrap());
        let communicator = Arc::new(DebugCommunicator::from(LC::from_config(config).await.unwrap(), None));
        loop {
            match communicator.receive_downlink(None).await {
                Ok(content) => {
                    for packet in content {
                        if !packet.transmission.payload.is_empty() {
                            println!("Received {} at sf {}",PrettyHexSlice(&packet.transmission.payload), packet.transmission.spreading_factor);
                            let mhdr = MHDR::from_bytes(packet.transmission.payload[0]);
    
                            let client_clone = Arc::clone(&client);
                            let radio_clone = Arc::clone(&communicator);
    
                            tokio::spawn(async move {
                                let answer = Self::dispatch_task(&mhdr, &packet.transmission.payload, &client_clone).await;
                                match answer {
                                    Ok(ans) => {
                                        if let Some((in_answer, dest)) = &ans {
                                            radio_clone.send_uplink(in_answer, None, Some(*dest)).await.unwrap();
                                        }
                                        match client_clone.create_uplink(&packet.transmission.payload, (ans.map(|v| v.0)).as_deref(), n_id).await {
                                            Ok(_) =>  {
                                                println!("uplink created successfully");
                                            }, 
                                            Err(e) => {
                                                eprintln!("Error creating uplink with answer: {e:?}")
                                            },
                                        };
                                    },
                                    Err(e) => eprintln!("Packet {}: {e:?}", PrettyHexSlice(&packet.transmission.payload)),
                                }
                            });
                        };
                    }
                },
                Err(e) => {
                    eprintln!("Error receiving downlink: {e:?}");
                    break;
                },
            }
        }
    }

    pub fn udp_routine<BC>(&self, config: &'static UDPNCConfig, blockchain_config: &BC::Config) -> JoinHandle<()> where BC: BlockchainClient + 'static  {
        let n_id: &str = self.n_id;
        //let m = self.mpsc_tx.clone();
        let c = blockchain_config.clone();

        tokio::spawn( async move {
            //TODO implement nc communications 
            //let nc_addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), self.config.tcp_nc_port);
            
            let client: Arc<BC> = Arc::new(*BC::from_config(&c).await.unwrap());
            let socket = UdpSocket::bind(format!("{}:{}",config.addr, config.port)).await.unwrap();

            let mut buf = Vec::with_capacity(512);
            while let Ok((bytes_read, addr)) = socket.recv_buf_from(&mut buf).await {
                //let mm = m.clone();
                let c = Arc::clone(&client);
                let data = buf[..bytes_read].to_vec();
                tokio::spawn(async move {
                    let mhdr = MHDR::from_bytes(data[0]);
                    let answer = Self::dispatch_task(&mhdr, &data, &c).await;
                    match answer {
                        Ok(ans) => {
                            if let Some((in_answer, _)) = &ans {
                                let s = UdpSocket::bind("0.0.0.0:0").await.unwrap();
                                s.send_to(in_answer, addr).await.unwrap();
                            }
                            match c.create_uplink(&data, (ans.map(|v| v.0)).as_deref(), n_id).await {
                                Ok(_) =>  {
                                    println!("uplink created successfully");
                                }, 
                                Err(e) => {
                                    eprintln!("Error creating uplink with answer: {e:?}")
                                },
                            };
                        },
                        Err(e) => eprintln!("Packet {}: {e:?}", PrettyHexSlice(&data)),
                    };
                });
            };
        })
    }

    pub fn routine<LC,BC>(&self, config: &'static LC::Config, bc_config: &'static BC::Config) -> JoinHandle<()> 
    where LC: LoRaWANCommunicator + 'static, BC: BlockchainClient + 'static {

        let n_id = self.n_id;

        tokio::spawn(async move {
            // Self::handle_colosseum_connection(mpsc_tx_clone, colosseum_config, n_id).await
            //Self::communicator_routine::<ColosseumCommunicator, BlockchainMockClient>(mpsc_tx_clone, colosseum_config, n_id, BlockchainMockClientConfig).await
            Self::communicator_routine::<LC, BC>(config, n_id, bc_config).await
        })
    }
}


/*
async fn handle_colosseum_connection(mpsc_tx: MpscSender<CommandWrapper>, colosseum_config: &'static ColosseumDeviceConfig, n_id: &'static str) {
        let client = Arc::new(BlockchainExeClient::new("orderer1.orderers.dlwan.phd:6050", "lorawan", "lorawan", None));
        let radio_communicator = Arc::new(*ColosseumCommunicator::from_config(colosseum_config).unwrap());

        loop {
            match radio_communicator.receive_downlink(None).await {
                Ok(mut content) => {
                    let packet = content.remove(&colosseum_config.radio_config.spreading_factor).unwrap();
                    if !packet.payload.is_empty() {
                        println!("Received {} at sf {}",PrettyHexSlice(&packet.payload),&colosseum_config.radio_config.spreading_factor.value());
                        let mhdr = MHDR::from_bytes(packet.payload[0]);

                        let m = mpsc_tx.clone();
                        let c = Arc::clone(&client);
                        let r = Arc::clone(&radio_communicator);

                        tokio::spawn(async move {
                            let answer = Self::dispatch_task(&mhdr, packet.payload.clone(), &m).await;
                            match answer {
                                Ok(ans) => {
                                    if let Some((in_answer, dest)) = &ans {
                                        r.send_uplink(in_answer, None, Some(*dest)).await.unwrap();
                                    }
                                    match c.create_uplink(&packet.payload, (ans.map(|v| v.0)).as_deref(), n_id).await {
                                        Ok(_) =>  {
                                            println!("uplink created successfully");
                                        }, 
                                        Err(e) => println!("Error creating uplink with answer: {e:?}")
,
                                    };
                                },
                                Err(e) => {
                                    println!("Error receiving answer from task: {e:?}")
                                },
                            }
                        });
                    };
                },
                Err(e) => {
                    println!("Error receiving downlin: {e:?}");
                    break;
                },
            }
        }
    }
    
    async fn handle_radio_connection(mpsc_tx: MpscSender<CommandWrapper>, radio_config: &'static RadioDeviceConfig, n_id: &'static str) {
        let client = Arc::new(BlockchainExeClient::new("orderer1.orderers.dlwan.phd:6050", "lorawan", "lorawan", None));
        let radio_communicator = Arc::new(*RadioCommunicator::from_config(radio_config).unwrap());
        loop {
            match radio_communicator.receive_downlink(None).await {
                Ok(mut content) => {
                    let packet = content.remove(&radio_config.spreading_factor).unwrap();
                    if !packet.payload.is_empty() {
                        println!("Received {} at sf {}",PrettyHexSlice(&packet.payload),&radio_config.spreading_factor.value());
                        let mhdr = MHDR::from_bytes(packet.payload[0]);

                        let mpsc_clone = mpsc_tx.clone();
                        let client_clone = Arc::clone(&client);
                        let radio_clone = Arc::clone(&radio_communicator);

                        tokio::spawn(async move {
                            let answer = Self::dispatch_task(&mhdr, packet.payload.clone(), &mpsc_clone).await;
                            match answer {
                                Ok(ans) => {
                                    if let Some((in_answer, dest)) = &ans {
                                        radio_clone.send_uplink(in_answer, None, Some(*dest)).await.unwrap();
                                    }
                                    match client_clone.create_uplink(&packet.payload, (ans.map(|v| v.0)).as_deref(), n_id).await {
                                        Ok(_) =>  {
                                            println!("uplink created successfully");
                                        }, 
                                        Err(e) => {
                                            println!("Error creating uplink with answer: {e:?}")
                                        },
                                    };
                                },
                                Err(e) => eprintln!("{e:?}"),
                            }
                        });
                    };
                },
                Err(e) => {
                    println!("Error receiving downlin: {e:?}");
                },
            }
        }
    }
*/