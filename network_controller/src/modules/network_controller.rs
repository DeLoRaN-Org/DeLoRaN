use std::sync::Arc;

use blockchain_api::{BlockchainClient, BlockchainError};
use consensus::{consensus_server::{ConsensusConfig, ConsensusServer}, ConsensusMessage};
use lorawan::{utils::{increment_nonce, nonce_valid, PrettyHexSlice, traits::ToBytesWithContext, errors::LoRaWANError}, device::Device, lorawan_packet::{LoRaWANPacket, join::{JoinAcceptPayload, JoinRequestType}, payload::Payload, mhdr::{MHDR, MType, Major}, mac_payload::MACPayload, fhdr::FHDR, fctrl::{FCtrl, DownlinkFCtrl}, mac_commands}};
use lorawan_device::{communicator::{ReceivedTransmission, Transmission}, configs::UDPNCConfig, devices::udp_device::UDPSender, split_communicator::SplitCommunicator};
use openssl::sha::sha256;

use tokio::{net::UdpSocket, sync::{mpsc::Sender, oneshot}, task::JoinHandle};
use crate::modules::error::NCError;
use super::downlink_scheduler::{DownlinkScheduler, DownlinkSchedulerMessage};
use lorawan_device::split_communicator::LoRaReceiver;

struct DownlinkConsensusLedgerUpdateInfo {
    dev_addr: [u8; 4],
    nc_list: Vec<String>,
}

struct DispatchResults {
    info: Option<DownlinkConsensusLedgerUpdateInfo>,
    answer: Option<Vec<u8>>,
}

#[derive(Clone)]
pub struct NetworkController {
    nc_id: &'static str,
    consensus_sender: Arc<Sender<ConsensusMessage>>,
}

impl NetworkController {
    pub fn new(nc_id: &'static str, consensus_config: ConsensusConfig) -> Self {

        let consensus_sender = ConsensusServer::run_instance(nc_id.to_string(), consensus_config).expect("Without consensus server the network controller cannot work");
        Self {
            nc_id,
            consensus_sender: Arc::new(consensus_sender)
        }
    }

    async fn handle_join_request(join_request: &[u8], bc_client: &Arc<impl BlockchainClient>, nc_id: &'static str) -> Result<DispatchResults, NCError> {
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

                        if bc_client.join_procedure(join_request, &join_accept, nc_id, jr_p.dev_eui()).await? {
                            println!("I'm the one who must send the join accept");
                            Ok(DispatchResults {
                                    info: None,
                                    answer: Some(join_accept),
                            })
                        } else {
                            Ok(DispatchResults {
                                info: None,
                                answer: None,
                            })
                        }
                        
                    }
                }
            }
        } else { Err(NCError::InvalidJoinRequest("Not a join request".to_string())) }
    }

    async fn handle_unconfirmed_data_up(data_up: &[u8], bc_client: &Arc<impl BlockchainClient>) -> Result<(Device, DispatchResults), NCError> {
        let packet = LoRaWANPacket::from_bytes(data_up, None, true)?;
        if let Payload::MACPayload(payload) = packet.into_payload() {
            let dev_addr = payload.fhdr().dev_addr();

            if let Ok(session) = bc_client.get_device_session(&dev_addr).await {
                let fcnt_u16 = payload.fhdr().fcnt();
                let current_fcnt = session.f_cnt_up;  
                        
                let (fcnt_up_valid, _fcnt_up_looped) = nonce_valid(fcnt_u16, current_fcnt as u16);
                if !fcnt_up_valid { return Err(NCError::InvalidUplink(format!("Invalid fcnt_up, expected > {current_fcnt}, received {fcnt_u16}"))); }

                let nc_list = session.nc_ids.clone();
                let device = session.into();

                let p = LoRaWANPacket::from_bytes(data_up, Some(&device), true)?; 
                if let Payload::MACPayload(_mp) = p.payload() {
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
                Ok((device, DispatchResults {
                    info: Some(DownlinkConsensusLedgerUpdateInfo {
                        dev_addr,
                        nc_list,
                    }),
                    answer: None,
                }))
            } else { Err(NCError::UnknownDevAddr(payload.fhdr().dev_addr())) }
        } else { Err(NCError::InvalidUplink("Not a MACPayload payload".to_string())) }
    }

    async fn handle_confirmed_data_up(data_up: &[u8], bc_client: &Arc<impl BlockchainClient>) -> Result<DispatchResults, NCError> {
        let (mut device, mut results) = Self::handle_unconfirmed_data_up(data_up, bc_client).await?;

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
        results.answer = Some(data_down);
        Ok(results)
    }
    
    async fn dispatch_task(mhdr: &MHDR, buf: &[u8], bc_client: &Arc<impl BlockchainClient>, nc_id: &'static str) -> Result<DispatchResults, NCError> {
        match mhdr.mtype() {
            MType::JoinRequest => {
                Self::handle_join_request(buf, bc_client, nc_id).await
            },
            MType::UnconfirmedDataUp => {
                let (_, results) = Self::handle_unconfirmed_data_up(buf, bc_client).await?;
                Ok(results)
            },
            MType::ConfirmedDataUp => {
                Self::handle_confirmed_data_up(buf, bc_client).await
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

    async fn consensus_round(consensus_send: &Sender<ConsensusMessage>, nc_list: Vec<String>, dev_addr: String, packet: &[u8], rssi: f32) -> Result<bool, NCError> {
        let (consensus_sender, consensus_receiver) = oneshot::channel();
        consensus_send.send(ConsensusMessage {
            nc_list,
            dev_addr,
            packet: packet.to_owned(),
            rssi: (rssi * 1000.0) as i32,
            response: consensus_sender,
        }).await.expect("If cannot send to consensus server, the network controller cannot work");
        consensus_receiver.await.map_err(|e| NCError::CommandTransmissionFailed(e.to_string()))
    }

    pub fn udp_routine<BC>(&self, config: &'static UDPNCConfig, blockchain_config: &BC::Config) -> JoinHandle<()> 
    where BC: BlockchainClient + 'static  {
        let nc_id: &str = self.nc_id;        
        let c = blockchain_config.clone();
        let consensus_sender = self.consensus_sender.clone();

        tokio::spawn( async move {
            let client: Arc<BC> = Arc::new(*BC::from_config(&c).await.unwrap());
            let socket = Arc::new(UdpSocket::bind(format!("{}:{}",config.addr, config.port)).await.unwrap());
            let mut buf = [0_u8; 512];

            let (downlink_sender, receiver) = tokio::sync::mpsc::channel(100);

            let udp_sender = UDPSender::new(socket.clone());
            let mut downlink_scheduler = DownlinkScheduler::new(udp_sender, receiver);
            tokio::spawn(async move {
                downlink_scheduler.run().await;
            });

            let downlink_sender = Arc::new(downlink_sender);
            while let Ok((bytes_read, addr)) = socket.recv_from(&mut buf).await {
                //println!("Content: {}", String::from_utf8_lossy(&buf[..bytes_read]));
                let transmission = serde_json::from_slice::<ReceivedTransmission>(&buf[..bytes_read]).unwrap();
                let just_arrived = tokio::time::Instant::now();

                let c = Arc::clone(&client);
                let dlsc = Arc::clone(&downlink_sender);
                let csc = Arc::clone(&consensus_sender);
                tokio::spawn(async move {
                    let data = &transmission.transmission.payload;
                    let mhdr = MHDR::from_bytes(data[0]);     
                    match Self::dispatch_task(&mhdr, data, &c, nc_id).await {
                        Ok(ans) => {
                            let should_downlink_and_update_ledger = if mhdr.is_join_rejoin() && ans.answer.is_some() {
                                true
                            } else if let Some(info) = ans.info {
                                match Self::consensus_round(&csc, info.nc_list , PrettyHexSlice(&info.dev_addr).to_string(), &transmission.transmission.payload, transmission.arrival_stats.rssi).await {
                                    Ok(v) => v,
                                    Err(e) => {
                                        eprintln!("Consensus error: {e:?}");
                                        false
                                    }
                                }
                            } else {
                                false
                            };

                            if should_downlink_and_update_ledger {
                                if let Some(v) = &ans.answer {
                                    let mut t = Transmission {
                                        frequency: transmission.transmission.frequency,
                                        bandwidth: transmission.transmission.bandwidth,
                                        spreading_factor: transmission.transmission.spreading_factor,
                                        code_rate: transmission.transmission.code_rate,
                                        uplink: false,
                                        payload: v.clone(),
                                        ..Default::default()
                                    };
                                    let bytes = serde_json::to_vec(&t).unwrap();
                                    t.payload = bytes;
    
                                    let downlink_message = DownlinkSchedulerMessage {
                                        transmission: t,
                                        moment: if mhdr.is_join_rejoin() {
                                            just_arrived + tokio::time::Duration::from_secs(5)
                                        } else {
                                            just_arrived + tokio::time::Duration::from_secs(1)
                                        },
                                        additional_info: Some(addr)
                                    };
                                    dlsc.send(downlink_message).await.unwrap();
                                }
                                if !mhdr.is_join_rejoin() {
                                    match c.create_uplink(data, ans.answer.as_deref()).await {
                                        Ok(_) =>  {
                                            println!("uplink created successfully");
                                        }, 
                                        Err(e) => {
                                            eprintln!("Error creating uplink with answer: {e:?}")
                                        },
                                    };
                                }
                            }
                        },                        
                        Err(e) => eprintln!("Packet {}: {e:?}", PrettyHexSlice(data)),
                    };
                });
            };
        })
    }

    async fn communicator_routine<LC,BC>(config: &'static LC::Config, nc_id: &'static str, blockchain_config: &BC::Config, consensus_sender: Arc<Sender<ConsensusMessage>>) 
    where LC: SplitCommunicator + 'static, 
          BC: BlockchainClient + 'static {
        
        let client: Arc<BC> = Arc::new(*BC::from_config(blockchain_config).await.unwrap());
        let (sender, receiver) = LC::from_config(config).await.unwrap().split_communicator().await.unwrap();
        
        let (downlink_sender, downlink_receiver) = tokio::sync::mpsc::channel(100);
        let downlink_sender = Arc::new(downlink_sender);
        
        let mut downlink_scheduler = DownlinkScheduler::new(sender, downlink_receiver);
        tokio::spawn(async move {
            downlink_scheduler.run().await;
        });

        loop {
            match receiver.receive(None).await {
                Ok(content) => {
                    for packet in content {
                        if !packet.transmission.payload.is_empty() {
                            println!("Received {} at sf {}",PrettyHexSlice(&packet.transmission.payload), packet.transmission.spreading_factor);
                            let just_arrived = tokio::time::Instant::now();

                            let mhdr = MHDR::from_bytes(packet.transmission.payload[0]);
    
                            let client_clone = Arc::clone(&client);
                            let csc = Arc::clone(&consensus_sender);
                            let dsc = Arc::clone(&downlink_sender);
    
                            tokio::spawn(async move {
                                match Self::dispatch_task(&mhdr, &packet.transmission.payload, &client_clone, nc_id).await {
                                    Ok(ans) => {
                                        //TODO fixare i parametri
                                        let should_downlink_and_update_ledger = if mhdr.is_join_rejoin() && ans.answer.is_some() {
                                            true
                                        } else if let Some(info) = ans.info {
                                            match Self::consensus_round(&csc, info.nc_list, PrettyHexSlice(&info.dev_addr).to_string(), &packet.transmission.payload, packet.arrival_stats.rssi).await {
                                                Ok(v) => v,
                                                Err(e) => {
                                                    eprintln!("Consensus error: {e:?}");
                                                    false
                                                }
                                            } 
                                        } else {
                                            false
                                        };
            
                                        if should_downlink_and_update_ledger {
                                            if let Some(v) = &ans.answer {
                                                let mut t = Transmission {
                                                    frequency: packet.transmission.frequency,
                                                    bandwidth: packet.transmission.bandwidth,
                                                    spreading_factor: packet.transmission.spreading_factor,
                                                    code_rate: packet.transmission.code_rate,
                                                    uplink: false,
                                                    payload: v.clone(),
                                                    ..Default::default()
                                                };
                                                let bytes = serde_json::to_vec(&t).unwrap();
                                                t.payload = bytes;
                
                                                let downlink_message = DownlinkSchedulerMessage {
                                                    transmission: t,
                                                    moment: if mhdr.is_join_rejoin() {
                                                        just_arrived + tokio::time::Duration::from_secs(5)
                                                    } else {
                                                        just_arrived + tokio::time::Duration::from_secs(1)
                                                    },
                                                    additional_info: None
                                                };
                                                dsc.send(downlink_message).await.unwrap();
                                            }
                                            if !mhdr.is_join_rejoin() {
                                                match client_clone.create_uplink(&packet.transmission.payload, ans.answer.as_deref()).await {
                                                    Ok(_) =>  {
                                                        println!("uplink created successfully");
                                                    }, 
                                                    Err(e) => {
                                                        eprintln!("Error creating uplink with answer: {e:?}")
                                                    },
                                                };
                                            }
                                        }
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

    pub fn routine<LC,BC>(&self, config: &'static LC::Config, bc_config: &'static BC::Config) -> JoinHandle<()> 
    where LC: SplitCommunicator + 'static, BC: BlockchainClient + 'static {
        tokio::spawn(Self::communicator_routine::<LC, BC>(config, self.nc_id, bc_config, self.consensus_sender.clone()))
    }
}
