mod utils;

use std::net::{Ipv4Addr, SocketAddr, IpAddr};

use blockchain_api::{exec_bridge::BlockchainExeClient, BlockchainError, BlockchainClient};
use fake_device::{communicators::{ColosseumCommunication, LoRaWANCommunication, RadioCommunication}, configs::{RadioDeviceConfig, ColosseumDeviceConfig}};
use lorawan::{utils::{increment_nonce, nonce_valid, PrettyHexSlice, traits::ToBytesWithContext, errors::LoRaWANError, eui::EUI64}, device::Device, lorawan_packet::{LoRaWANPacket, join::{JoinAcceptPayload, JoinRequestType}, payload::Payload, mhdr::{MHDR, MType, Major}, mac_payload::MACPayload, fhdr::FHDR, fctrl::{FCtrl, DownlinkFCtrl}, mac_commands}};
use serde::{Serialize, Deserialize};

use tokio::{net::{TcpListener, TcpStream}, io::{AsyncReadExt, AsyncWriteExt}};
use tokio::sync::mpsc::{self, Sender as MpscSender};
use crate::{utils::error::NCError, network_controller::utils::{NCTaskResponse, NCTaskCommand}};

use self::utils::CommandWrapper;


#[derive(Copy, Clone)]
pub struct NetworkController {
    pub n_id: &'static str,
    tcp_config:   Option<&'static NetworkControllerTCPConfig>,
    radio_config: Option<&'static RadioDeviceConfig>,
    colosseum_config: Option<&'static ColosseumDeviceConfig>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct NetworkControllerTCPConfig {
    pub tcp_dev_port: u16,
    pub tcp_nc_port: u16,
}

impl NetworkController {
    pub fn init(n_id: &'static str, tcp_config: Option<&'static NetworkControllerTCPConfig>, radio_config: Option<&'static RadioDeviceConfig>, colosseum_config: Option<&'static ColosseumDeviceConfig>) -> Self {
        Self {
            n_id,
            tcp_config,
            radio_config,
            colosseum_config,
        } 
    }

    async fn handle_join_request(join_request: Vec<u8>, bc_client: &impl BlockchainClient) -> Result<(Vec<u8>, EUI64), NCError> {
        let packet = LoRaWANPacket::from_bytes(&join_request, None, true)?;
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
                        LoRaWANPacket::validate_mic(&join_request, &packet, &device)?;
                        
                        let dev_addr = [0_u8; 4].map(|_| rand::random::<u8>()); 
                        let mut dl_settings = 0_u8;
                        let opt_neg_v1_1 = 0b10000000;
                        dl_settings |= 0b00010001; //TODO per ora rx1_dr_offset 1, rx2_data_rate 1, capire come farle bene poi
                        if device.version().is_1_1_or_greater() {
                            dl_settings |= opt_neg_v1_1
                        }
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
                        device.derive_session_context(&join_accept)?;
                        device.set_last_join_request_received(JoinRequestType::JoinRequest);

                        let packet = LoRaWANPacket::new(MHDR::new(MType::JoinAccept, Major::R1), Payload::JoinAccept(join_accept));
                        println!("{device}");
                        
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

    async fn handle_unconfirmed_data_up(data_up: &[u8], bc_client: &impl BlockchainClient) -> Result<Device, NCError> {
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

                if let Payload::MACPayload(mp) = p.payload() {
                    println!("{:?}", String::from_utf8_lossy(mp.frm_payload().unwrap()));
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

    async fn handle_confirmed_data_up(data_up: &[u8], bc_client: &impl BlockchainClient ) -> Result<(Vec<u8>, EUI64), NCError> {
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

    async fn handle_commands_task(mut mpsc_rx: mpsc::Receiver<CommandWrapper>) {
        let client = BlockchainExeClient::new("orderer1.orderers.dlwan.phd:6050", "lorawan", "lorawan", None);
        while let Some(command) = mpsc_rx.recv().await {
            let client_cloned = client.clone();
            tokio::spawn(async move {
                if (match command.0 {
                    NCTaskCommand::JoinRequest { join_request } => {
                        let response = Self::handle_join_request(join_request, &client_cloned).await;
                        command.1.send(NCTaskResponse::JoinRequest { result: response })
                        
                    },
                    NCTaskCommand::UnConfirmedDataUp { data_up } => {
                        let response = Self::handle_unconfirmed_data_up(&data_up, &client_cloned).await;
                        command.1.send(NCTaskResponse::UnConfirmedDataUp { result: response.map(|_| ()) })
                    },
                    NCTaskCommand::ConfirmedDataUp   { data_up } => {
                        let response = Self::handle_confirmed_data_up(&data_up, &client_cloned).await;
                        command.1.send(NCTaskResponse::ConfirmedDataUp { result: response })
                    },
                }).is_err() { eprintln!("Error sending response back to task handler") }
            }); //end task
        }
    }
    
    async fn create_and_send_task(mhdr: &MHDR, buf: &[u8], mpsc_tx: &MpscSender<CommandWrapper>) -> Result<Option<(Vec<u8>, EUI64)>, NCError> {
        match mhdr.mtype() {
            MType::JoinRequest => {
                let cmd = NCTaskCommand::JoinRequest { join_request: buf.to_vec() };
                if let Ok(NCTaskResponse::JoinRequest { result }) = utils::send_task(cmd, mpsc_tx).await {
                    result.map(Some)
                } else {
                    eprintln!("Error while sending command to task");
                    Err(NCError::CommandTransmissionFailed("should not happen".to_string()))
                }
                //None
            },
            MType::UnconfirmedDataUp => {
                let data_up = buf;
                let data_up_vect = data_up.to_vec();

                let cmd = NCTaskCommand::UnConfirmedDataUp { data_up: data_up_vect };
                utils::send_task(cmd, mpsc_tx).await.map(|_v| None)
            },
            MType::ConfirmedDataUp => {
                let data_up = buf;
                let data_up_vect = data_up.to_vec();

                let cmd = NCTaskCommand::ConfirmedDataUp { data_up: data_up_vect };
                match utils::send_task(cmd, mpsc_tx).await {
                    Ok(nctr) => {
                        if let NCTaskResponse::ConfirmedDataUp { result } = nctr {
                            result.map(Some)
                        }
                        else { Err(NCError::CommandTransmissionFailed("should not happen".to_string())) }
                    },
                    Err(e) => Err(e),
                }
            },
            MType::RejoinRequest => {
                unimplemented!("RejoinRequest")
            },
            MType::UnconfirmedDataDown => todo!(),
            MType::ConfirmedDataDown => todo!(),
            
            MType::Proprietary => todo!(),
            MType::JoinAccept => todo!(), //-> join accept ricevuta dal NC? da ignorare?
        }
    }

    async fn handle_tcp_connection(mut cl_sock: TcpStream, mpsc_tx: MpscSender<CommandWrapper>, n_id: &'static str) {
        let client = BlockchainExeClient::new("orderer1.orderers.dlwan.phd:6050", "lorawan", "lorawan", None);
        let mut buf = [0_u8; 1024];
        while let Ok(bytes_read) = cl_sock.read(&mut buf).await {
            println!("read {} bytes: {}", bytes_read, PrettyHexSlice(&buf[..bytes_read]));
            if bytes_read == 0 {break}
            let mhdr = MHDR::from_bytes(buf[0]);
            let answer = Self::create_and_send_task(&mhdr, &buf[..bytes_read], &mpsc_tx).await;
            match answer {
                Ok(ans) => {
                    //TODO UNCONFIRMED DATA UP NON SEGNALA SE IL PACCHETTO NON È VALIDO, RITORNARE UN CAMPO RESULT INVECE CHE NULLA
                    if let Some((in_answer, _)) = &ans {
                        cl_sock.write_all(in_answer).await.unwrap();
                    }
                    match client.create_uplink(&buf[..bytes_read], (ans.map(|v| v.0)).as_deref(), n_id).await {
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
        }
        println!("task ended");
    }

    async fn handle_colosseum_connection(mpsc_tx: MpscSender<CommandWrapper>, colosseum_config: &'static ColosseumDeviceConfig, sdr_code: &'static str, n_id: &'static str) {
        let client = BlockchainExeClient::new("orderer1.orderers.dlwan.phd:6050", "lorawan", "lorawan", None);

        /*let mut radio_communicator = ColosseumCommunication::new(IpAddr::V4(Ipv4Addr::LOCALHOST), RadioDeviceConfig {
            region: Region::EU863_870,
            spreading_factor: SpreadingFactor::new(7),
            data_rate: DataRate::new(5),
            rx_gain: 10,
            tx_gain: 20,
            bandwidth: 125_000,
            sample_rate: 1_000_000,
            rx_freq: 990_000_000,
            tx_freq: 1_010_000_000,
            rx_chan_id: 0,
            tx_chan_id: 1,
        }, sdr_code);*/

        let mut radio_communicator = ColosseumCommunication::new(colosseum_config.address, colosseum_config.radio_config, sdr_code);

        loop {
            match radio_communicator.receive_downlink(None, None).await {
                Ok(content) => {
                    let packet = content.get(&colosseum_config.radio_config.spreading_factor).cloned().unwrap();
                    if !packet.payload.is_empty() {
                        println!("Received {} at sf {}",PrettyHexSlice(&packet.payload),&colosseum_config.radio_config.spreading_factor.value());
                        let mhdr = MHDR::from_bytes(packet.payload[0]);

                        let mpsc_clone = mpsc_tx.clone();
                        let client_clone = client.clone();
                        let mut radio_communicator_clone = radio_communicator.clone();
                        tokio::spawn(async move {
                            let answer = Self::create_and_send_task(&mhdr, &packet.payload, &mpsc_clone).await;
                            match answer {
                                Ok(ans) => {
                                    //TODO UNCONFIRMED DATA UP NON SEGNALA SE IL PACCHETTO NON È VALIDO, RITORNARE UN CAMPO RESULT INVECE CHE NULLA
                                    if let Some((in_answer, dest)) = &ans {
                                        radio_communicator_clone.send_uplink(in_answer, None, Some(*dest)).await.unwrap();
                                    }
                                    match client_clone.create_uplink(&packet.payload, (ans.map(|v| v.0)).as_deref(), n_id).await {
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
        let client = BlockchainExeClient::new("orderer1.orderers.dlwan.phd:6050", "lorawan", "lorawan", None);

        /*let mut radio_communicator = RadioCommunication::new(RadioDeviceConfig {
            region: Region::EU863_870,
            spreading_factor: SpreadingFactor::new(7),
            data_rate: DataRate::new(5),
            rx_gain: 10,
            tx_gain: 20,
            bandwidth: 125_000,
            sample_rate: 1_000_000,
            rx_freq: 990_000_000,
            tx_freq: 1_010_000_000,
            rx_chan_id: 0,
            tx_chan_id: 1,
        });*/

        let mut radio_communicator = RadioCommunication::new(*radio_config);
        
        loop {
            match radio_communicator.receive_downlink(None, None).await {
                Ok(content) => {
                    let packet = content.get(&radio_config.spreading_factor).cloned().unwrap();
                    if !packet.payload.is_empty() {
                        println!("Received {} at sf {}",PrettyHexSlice(&packet.payload),&radio_config.spreading_factor.value());
                        let mhdr = MHDR::from_bytes(packet.payload[0]);

                        let mpsc_clone = mpsc_tx.clone();
                        let client_clone = client.clone();
                        let mut radio_communicator_clone = radio_communicator;
                        tokio::spawn(async move {
                            let answer = Self::create_and_send_task(&mhdr, &packet.payload, &mpsc_clone).await;
                            match answer {
                                Ok(ans) => {
                                    //TODO UNCONFIRMED DATA UP NON SEGNALA SE IL PACCHETTO NON È VALIDO, RITORNARE UN CAMPO RESULT INVECE CHE NULLA
                                    if let Some((in_answer, dest)) = &ans {
                                        radio_communicator_clone.send_uplink(in_answer, None, Some(*dest)).await.unwrap();
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

    pub async fn routine(&self, sdr_code: Option<&'static str>) -> Result<(), NCError> {
        let (mpsc_tx, mpsc_rx) = mpsc::channel::<CommandWrapper>(128);
        
        let n_id = self.n_id;
        tokio::spawn(async move {
            Self::handle_commands_task(mpsc_rx).await
        });

        let mut initialized = false;

        let c_spawn = if let (Some(sdr_code), Some(colosseum_config)) = (sdr_code, self.colosseum_config) {
            initialized = true;
            let mpsc_tx_clone = mpsc_tx.clone();
            Some(tokio::spawn( async move {
                Self::handle_colosseum_connection(mpsc_tx_clone, colosseum_config, sdr_code, n_id).await
            }))
        } else {
            None
        };
        
        let r_spawn = if let Some(radio_config) = self.radio_config {
            initialized = true;
            let mpsc_tx_clone = mpsc_tx.clone();
            Some(tokio::spawn( async move {
                Self::handle_radio_connection(mpsc_tx_clone, radio_config, n_id).await
            }))
        } else {
            None
        };
        
        if let Some(tcp_config) = self.tcp_config {
            initialized = true;
            
            //TODO let nc_addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), self.config.tcp_nc_port);
            let dev_addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), tcp_config.tcp_dev_port);

            let dev_socket = TcpListener::bind(dev_addr).await.unwrap();
            println!("Waiting for connections...");
            while let Ok((cl_sock, _addr)) = dev_socket.accept().await {
                let mpsc_tx_clone = mpsc_tx.clone();
                println!("Received connection");
                tokio::spawn(async move {
                    Self::handle_tcp_connection(cl_sock, mpsc_tx_clone, n_id).await
                });
            };
        }
        assert!(initialized,"No valid configuration provided");

        if let Some(h) = c_spawn {
            let r = h.await;
            if let Err(e) = r {
                eprintln!("{e:?}");
            }
        }
        if let Some(h) = r_spawn {
            let r = h.await;
            if let Err(e) = r {
                eprintln!("{e:?}");
            }
        }

        Ok(())
    }
}