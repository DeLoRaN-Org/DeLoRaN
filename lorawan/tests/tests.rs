#[cfg(test)]
mod tests {
    use hex::FromHex;
    use lorawan::{
        device::{
            session_context::{ApplicationSessionContext, NetworkSessionContext, SessionContext},
            Device, DeviceClass, LoRaWANVersion,
        },
        encryption::{
            aes_128_cmac, aes_128_decrypt, aes_128_decrypt_with_padding,
            aes_128_encrypt_with_padding, extract_mic, key::Key,
        },
        lorawan_packet::{
            fctrl::{DownlinkFCtrl, FCtrl, UplinkFCtrl},
            fhdr::FHDR,
            join::{
                JoinAcceptPayload, JoinRequestPayload, JoinRequestType, ReJoinRequest02,
                ReJoinRequest1, RejoinRequestPayload,
            },
            mac_commands::{EDMacCommands, NCMacCommands},
            mac_payload::MACPayload,
            mhdr::{MType, Major, MHDR},
            payload::Payload,
            LoRaWANPacket,
        },
        utils::{self, traits::ToBytesWithContext},
        utils::traits::ToBytes,
        utils::{errors::LoRaWANError, eui::EUI64, PrettyHexSlice},
    };
    use std::{convert::TryInto, panic};

    fn create_uninitialized_device() -> Device {
        let is_unidata = true;
        if is_unidata {
            Device::new(
                DeviceClass::A,
                None,
                EUI64::from_hex("50DE2646F9A7AC8E").unwrap(),
                EUI64::from_hex("DCBC65F607A47DEA").unwrap(),
                Key::from_hex("BBF326BE9AC051453AA616410F110EE7").unwrap(),
                Key::from_hex("BBF326BE9AC051453AA616410F110EE7").unwrap(),
                LoRaWANVersion::V1_0_2,
            )
        } else {
            Device::new(
                DeviceClass::A,
                None,
                EUI64::from_hex("70B3D57ED004E5E6").unwrap(),
                EUI64::from_hex("0102030405060708").unwrap(),
                Key::from_hex("2A1454172EEA9C137A0E8D1B6FC494E0").unwrap(),
                Key::from_hex("2A1454172EEA9C137A0E8D1B6FC494E0").unwrap(),
                LoRaWANVersion::V1_0_2,
            )
        }
    }

    fn create_initialized_device() -> Device {
        let mut device = Device::new(
            DeviceClass::A,
            None,
            EUI64::from_hex("50DE2646F9A7AC8E").unwrap(),
            EUI64::from_hex("DCBC65F607A47DEA").unwrap(),
            Key::from_hex("BBF326BE9AC051453AA616410F110EE7").unwrap(),
            Key::from_hex("BBF326BE9AC051453AA616410F110EE7").unwrap(),
            LoRaWANVersion::V1_1,
        );

        let network_context = NetworkSessionContext::new(
            Key::from_hex("75C3EB8BA73C9A0D5F74BB3E02E7EF9E").unwrap(),
            Key::from_hex("75C3EB8BA73C9A0D5F74BB3E02E7EF9E").unwrap(),
            Key::from_hex("75C3EB8BA73C9A0D5F74BB3E02E7EF9E").unwrap(),
            [0x60, 0x00, 0x08],
            [0xe0, 0x11, 0x3B, 0x2A],
            0,
            1,
            0,
        );

        let application_context = ApplicationSessionContext::new(
            Key::from_hex("5560CC0B0DC37BEBBFB39ACD337DD34D").unwrap(),
            0,
        );

        device.set_activation_abp(SessionContext::new(application_context, network_context));
        device
    }

    #[test]
    fn cmac() {
        let key = Key::from([
            0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
            0x80, 0x80,
        ]);
        let text = "ciao mamma guarda come mi diverto";
        let aes_128_ecb_cmac = "3bd793fa0a81e023ced3eb72719d7eed";
        
        match aes_128_cmac(&key, text.as_bytes()) {
            Ok(v) => {
                let s =  format!("{}",PrettyHexSlice(&v));
                assert_eq!(aes_128_ecb_cmac, s);
            }
            Err(err) => {
                panic!("{:?}", err);
            }
        }
    }

    #[test]
    fn aes_128_encrypt_decrypt() {
        let key = Key::from([
            0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
            0x80, 0x80,
        ]);

        let mut original_string = String::from("plain data to encrypt").into_bytes();
        utils::pad_to_16(&mut original_string);
        let mut data_to_encrypt = original_string.clone();
        match aes_128_encrypt_with_padding(&key, &mut data_to_encrypt) {
            Ok(v) => match aes_128_decrypt(&key, &v) {
                Ok(v) => {
                    let s = String::from_utf8_lossy(&v).to_string().into_bytes();
                    assert_eq!(s, original_string);
                }
                Err(e) => {
                    panic!("Error while decrypting {}", e)
                }
            },
            Err(e) => {
                panic!("Error while encrypting {}", e)
            }
        }
    }

    #[test]
    fn aes_128_encrypt_decrypt_2() {
        let k = Key::from_hex("BBF326BE9AC051453AA616410F110EE7").unwrap();

        let s1 = " ciao ";
        let mut enc = aes_128_encrypt_with_padding(&k, &mut Vec::from(s1.as_bytes())).unwrap();
        let mut dec = aes_128_decrypt_with_padding(&k, &mut enc).unwrap();
        let mut dec = aes_128_decrypt_with_padding(&k, &mut dec).unwrap();
        let enc = aes_128_encrypt_with_padding(&k, &mut dec).unwrap();

        let s2 = String::from_utf8_lossy(&enc);
        let s2 = s2.trim_end_matches('\0');
        println!("#{}#-#{}#", s1.len(), s2.len());
        println!("#{s1}#-#{s2}#");
        assert!(s1 == s2);
    }

    #[test]
    fn calculate_mic() {
        let phy_payload =
            Vec::from_hex(String::from("00EA7DA407F665BCDC8EACA7F94626DE50D1EC")).unwrap();
        let mic = Vec::from_hex(String::from("AB7C2E50")).unwrap();
        let key = Key::from_hex("BBF326BE9AC051453AA616410F110EE7").unwrap();

        let calculated_mic = extract_mic(&key, &phy_payload).unwrap();

        println!("{mic:?} - {calculated_mic:?}");
        assert_eq!(mic, calculated_mic)
    }

    #[test]
    fn key_derivation() {
        let mut device = create_uninitialized_device();

        device.set_dev_nonce(9138);
        println!("dev_nonce: {}", device.dev_nonce());

        let packet =
            Vec::from_hex("2076281796279C3FF432A37FAA6791C806E9278DDA0A629E149C96978F57C0FE36")
                .unwrap();

        let join_accept = LoRaWANPacket::from_bytes(&packet, Some(&device), false).unwrap();

        if let Payload::JoinAccept(ja) = join_accept.payload() {
            if let Err(e) = device.derive_session_context(ja) {
                match e {
                    LoRaWANError::SessionContextMissing => panic!("Missing JoinRequest context"),
                    LoRaWANError::OpenSSLErrorStack(e) => panic!("{}", e),
                    e => panic!("{:?}", e),
                }
            } else {
                eprintln!("{device}");

                let session_context = device.session().unwrap();

                let expected_nskey = "75C3EB8BA73C9A0D5F74BB3E02E7EF9E".to_ascii_lowercase();
                let expected_askey = "5560CC0B0DC37BEBBFB39ACD337DD34D".to_ascii_lowercase();
                eprintln!("{session_context}");
                assert_eq!(
                    session_context
                        .network_context()
                        .nwk_s_enc_key()
                        .to_hex()
                        .to_ascii_lowercase(),
                    expected_nskey
                );
                assert_eq!(
                    session_context
                        .application_context()
                        .app_s_key()
                        .to_hex()
                        .to_ascii_lowercase(),
                    expected_askey
                );
            }
        } else {
            panic!("Not a join accept")
        }
    }

    #[test]
    fn unconfirmed_data_down() {
        let device = create_initialized_device();
        let dev_addr = device.session().unwrap().network_context().dev_addr();

        let mhdr = MHDR::new(MType::UnconfirmedDataDown, Major::R1);

        let fctrl = FCtrl::Downlink(DownlinkFCtrl::new(false, false, true, false, 0));

        let fport = Some(1);
        //let fport = None;

        let mut fhdr = FHDR::new(*dev_addr, fctrl);
        fhdr.set_fcnt_from_device(&device, fport).unwrap();
        let frm_payload = Some(Vec::from("ciao mamma guarda come mi diverto"));
        //let frm_payload = None;

        let payload = MACPayload::new(fhdr, fport, frm_payload);

        let mut packet = LoRaWANPacket::default();
        packet.set_mhdr(mhdr);
        packet.set_payload(Payload::MACPayload(payload));

        let buffer = packet.to_bytes_with_context(&device).unwrap();

        println!("{}", PrettyHexSlice(&buffer));
    }

    #[test]
    fn unconfirmed_data_up() {
        let device = create_initialized_device();
        let dev_addr = device.session().unwrap().network_context().dev_addr();

        let mhdr = MHDR::new(MType::UnconfirmedDataUp, Major::R1);
        let mut payload = MACPayload::default();

        let fctrl = FCtrl::Uplink(UplinkFCtrl::new(true, false, true, false, 0));

        let fport = Some(1);

        let mut fhdr = FHDR::new(*dev_addr, fctrl);
        fhdr.set_fcnt_from_device(&device, fport).unwrap();
        payload.set_fhdr(fhdr);
        payload.set_fport(fport);
        payload.set_frm_payload(Some(Vec::from("ciao mamma guarda come mi diverto")));

        let mut packet = LoRaWANPacket::default();
        packet.set_mhdr(mhdr);
        packet.set_payload(Payload::MACPayload(payload));

        //let v = payload.to_bytes_with_context(&device).unwrap();
        //println!("{}", String::from_utf8_lossy(&v));

        let buffer = packet.to_bytes_with_context(&device).unwrap();

        println!("{}", PrettyHexSlice(&buffer));
    }

    #[test]
    fn join_accept() {
        let mut device = create_uninitialized_device();

        let mhdr = MHDR::new(MType::JoinAccept, Major::R1);

        //let dl_settings = 0b10010001;
        let dl_settings = 0b10000000;

        let join_nonce: [u8; 3] = Vec::from_hex("AA0845").unwrap().try_into().unwrap();

        let mut home_net_id = [0; 3];
        home_net_id.copy_from_slice(&Vec::from_hex("600008").unwrap());

        let mut dev_addr = [0; 4];
        dev_addr.copy_from_slice(&Vec::from_hex("E0103718").unwrap());

        let mut cf_list = [0; 16];
        cf_list.copy_from_slice(&Vec::from_hex("184F84B85E84886684586E84E8568400").unwrap());

        let join_accept = JoinAcceptPayload::new(
            JoinRequestType::JoinRequest,
            join_nonce,
            home_net_id,
            dev_addr,
            dl_settings,
            0x1,
            Some(cf_list),
        );

        let packet = LoRaWANPacket::new(mhdr, Payload::JoinAccept(join_accept));

        if let Payload::JoinAccept(ja_payload) = packet.payload() {
            device.derive_session_context(ja_payload).unwrap();
        };

        println!("{device}");

        let packet_bytes = packet.to_bytes_with_context(&device);

        match packet_bytes {
            Ok(p) => {
                println!("{:?}, {}", p, p.len());
                let s = PrettyHexSlice(&p).to_string();
                println!("{}, {}", s, s.len());
            }
            Err(e) => eprintln!("{e:?}"),
        }
    }

    #[test]
    fn join_request() {
        let mut device = create_uninitialized_device();

        let mhdr = MHDR::new(MType::JoinRequest, Major::R1);
        let payload = JoinRequestPayload::new(
            *device.join_eui(),
            *device.dev_eui(),
            device.dev_nonce_autoinc() as u16,
        );
        let packet = LoRaWANPacket::new(mhdr, Payload::JoinRequest(payload));
        let content = packet.to_bytes_with_context(&device).unwrap();

        println!("{}", PrettyHexSlice(&content));
    }

    #[test]
    fn confirmed_data_down() {
        let device = create_initialized_device();
        let dev_addr = device.session().unwrap().network_context().dev_addr();

        let mhdr = MHDR::new(MType::ConfirmedDataDown, Major::R1);
        let mut payload = MACPayload::default();

        let fctrl = FCtrl::Downlink(DownlinkFCtrl::new(false, false, true, false, 0));

        let fport = Some(1);

        let mut fhdr = FHDR::new(*dev_addr, fctrl);
        fhdr.set_fcnt_from_device(&device, fport).unwrap();
        payload.set_fhdr(fhdr);
        payload.set_fport(fport);
        payload.set_frm_payload(Some(Vec::from("ciao mamma guarda come mi diverto")));

        let mut packet = LoRaWANPacket::default();
        packet.set_mhdr(mhdr);
        packet.set_payload(Payload::MACPayload(payload));

        //let v = payload.to_bytes_with_context(&device).unwrap();
        //println!("{}", String::from_utf8_lossy(&v));

        let buffer = packet.to_bytes_with_context(&device).unwrap();

        println!("{}", PrettyHexSlice(&buffer));
    }

    #[test]
    fn confirmed_data_up() {
        let device = create_initialized_device();
        let dev_addr = device.session().unwrap().network_context().dev_addr();

        let mhdr = MHDR::new(MType::ConfirmedDataUp, Major::R1);
        let mut payload = MACPayload::default();

        let fctrl = FCtrl::Uplink(UplinkFCtrl::new(true, false, true, false, 0));

        let fport = Some(1);

        let mut fhdr = FHDR::new(*dev_addr, fctrl);
        fhdr.set_fcnt_from_device(&device, fport).unwrap();
        payload.set_fhdr(fhdr);
        payload.set_fport(fport);
        payload.set_frm_payload(Some(Vec::from("ciao mamma guarda come mi diverto")));

        let mut packet = LoRaWANPacket::default();
        packet.set_mhdr(mhdr);
        packet.set_payload(Payload::MACPayload(payload));

        //let v = payload.to_bytes_with_context(&device).unwrap();
        //println!("{}", String::from_utf8_lossy(&v));

        let buffer = packet.to_bytes_with_context(&device).unwrap();

        println!("{}", PrettyHexSlice(&buffer));
    }

    #[test]
    fn rejoin1() {
        let device = create_initialized_device();

        let mhdr = MHDR::new(MType::RejoinRequest, Major::R1);

        let rj1 = ReJoinRequest1::new(
            *device.join_eui(),
            *device.dev_eui(),
            device.join_context().rj_count1(),
        );
        let rejoin = RejoinRequestPayload::T1(rj1);
        let payload = Payload::RejoinRequest(rejoin);

        let packet = LoRaWANPacket::new(mhdr, payload);

        let buffer = packet.to_bytes_with_context(&device).unwrap();
        println!("{}", PrettyHexSlice(&buffer));
    }

    #[test]
    fn rejoin02() {
        let device = create_initialized_device();

        let mhdr = MHDR::new(MType::RejoinRequest, Major::R1);

        let rj1 = ReJoinRequest02::new(
            true,
            [10, 10, 10],
            *device.dev_eui(),
            device.session().unwrap().network_context().rj_count0(),
        );
        let rejoin = RejoinRequestPayload::T02(rj1);
        let payload = Payload::RejoinRequest(rejoin);

        let packet = LoRaWANPacket::new(mhdr, payload);

        let buffer = packet.to_bytes_with_context(&device).unwrap();
        println!("{}", PrettyHexSlice(&buffer));
    }

    #[test]
    fn ed_mac_commands_payload() {
        let device = create_initialized_device();
        let session = device.session().unwrap();
        let mhdr = MHDR::new(MType::UnconfirmedDataUp, Major::R1);
        let fctrl = FCtrl::Uplink(UplinkFCtrl {
            adr: false,
            ack: false,
            f_opts_len: 0,
            adr_ack_req: false,
            class_b: false,
        });
        let fhdr = FHDR::new(*session.network_context().dev_addr(), fctrl);

        let list = vec![
            EDMacCommands::ResetInd(1),
            EDMacCommands::DeviceTimeReq,
            EDMacCommands::RXParamSetupAns {
                rx1_dr_offset_ack: true,
                rx2_data_rate_ack: false,
                channel_ack: false,
            },
            EDMacCommands::DevStatusAns {
                battery: 200,
                margin: 10,
            },
            EDMacCommands::DlChannelAns {
                uplink_frequency_exists: true,
                channel_frequency_ok: true,
            },
        ];

        let payload_content = list
            .iter()
            .map(|elem| {
                println!("{:?}", elem.to_bytes());
                elem.to_bytes()
            })
            .reduce(|mut acc, elem| {
                acc.extend_from_slice(&elem);
                acc
            })
            .unwrap();

        let payload = MACPayload::new(fhdr, Some(0), Some(payload_content));
        let packet = LoRaWANPacket::new(mhdr, Payload::MACPayload(payload))
            .to_bytes_with_context(&device)
            .unwrap();
        println!("{}", PrettyHexSlice(&packet));
    }

    #[test]
    fn ns_mac_commands_payload_() {
        let device = create_initialized_device();
        let session = device.session().unwrap();
        let mhdr = MHDR::new(MType::UnconfirmedDataDown, Major::R1);
        let fctrl = FCtrl::Downlink(DownlinkFCtrl {
            adr: false,
            rfu: false,
            f_pending: false,
            ack: false,
            f_opts_len: 0,
        });
        let fhdr = FHDR::new(*session.network_context().dev_addr(), fctrl);

        let list = vec![NCMacCommands::NewChannelReq {
            ch_index: 1,
            freq: 878000,
            max_dr: 3,
            min_dr: 5,
        }];

        let payload_content = list
            .iter()
            .map(|elem| {
                println!("{:?}", elem.to_bytes());
                elem.to_bytes()
            })
            .reduce(|mut acc, elem| {
                acc.extend_from_slice(&elem);
                acc
            })
            .unwrap();

        let payload = MACPayload::new(fhdr, Some(0), Some(payload_content));
        let packet = LoRaWANPacket::new(mhdr, Payload::MACPayload(payload))
            .to_bytes_with_context(&device)
            .unwrap();
        println!("{}", PrettyHexSlice(&packet));
    }

    #[test]
    fn default_packet() {
        let device = create_initialized_device();
        let packet = LoRaWANPacket::default()
            .to_bytes_with_context(&device)
            .unwrap();
        println!("{}", PrettyHexSlice(&packet));
    }

    #[test]
    fn from_bytes_default_packet() {
        let device = create_initialized_device();
        let packet = LoRaWANPacket::default();

        println!("{packet:?}");

        let packet_1 = packet.to_bytes_with_context(&device).unwrap();


        let packet_2 = LoRaWANPacket::from_bytes(&packet_1, Some(&device), false)
            .unwrap()
            .to_bytes_with_context(&device)
            .unwrap();

        println!(
            "{} -- {}",
            PrettyHexSlice(&packet_1),
            PrettyHexSlice(&packet_2)
        )
    }

    #[test]
    fn from_bytes_unconfirmed_data_down() {
        let device = create_initialized_device();
        let dev_addr = device.session().unwrap().network_context().dev_addr();

        let mhdr = MHDR::new(MType::UnconfirmedDataDown, Major::R1);

        let fctrl = FCtrl::Downlink(DownlinkFCtrl::new(false, false, true, false, 0));

        let fport = Some(1);
        //let fport = None;

        let mut fhdr = FHDR::new(*dev_addr, fctrl);
        fhdr.set_fcnt_from_device(&device, fport).unwrap();
        let frm_payload = Some(Vec::from("ciao mamma guarda come mi diverto"));
        //let frm_payload = None;

        let payload = MACPayload::new(fhdr, fport, frm_payload);

        let mut packet = LoRaWANPacket::default();
        packet.set_mhdr(mhdr);
        packet.set_payload(Payload::MACPayload(payload));

        let packet_1 = packet.to_bytes_with_context(&device).unwrap();

        let npack = LoRaWANPacket::from_bytes(&packet_1, Some(&device), false).unwrap();

        //println!("{:?}", packet);
        //println!("{:?}", npack);

        let packet_2 = npack.to_bytes_with_context(&device).unwrap();

        let phs1 = PrettyHexSlice(&packet_1);
        let phs2 = PrettyHexSlice(&packet_2);
        //println!("{} -- {}", phs1, phs2);
        assert_eq!(phs1, phs2)
    }

    #[test]
    fn from_bytes_join_request() {
        let device = create_uninitialized_device();

        //let mhdr = MHDR::new(MType::JoinRequest, Major::R1);
        //let payload = JoinRequestPayload::new(
        //    *device.join_eui(),
        //    *device.dev_eui(),
        //    device.dev_nonce_autoinc() as u16,
        //);
        //let packet = LoRaWANPacket::new(mhdr, Payload::JoinRequest(payload));
        //
        //let packet_1 = packet.to_bytes_with_context(&device).unwrap();
        //println!("{}", PrettyHexSlice(&packet_1));


        let packet_1 = Vec::from_hex("00EA7DA407F665BCDC8EACA7F94626DE50B223B47ECCF8").unwrap(); //real packet uniorchestra

        let packet_2 = LoRaWANPacket::from_bytes(&packet_1, Some(&device), true).unwrap();

        println!("{packet_2:?}");
    }

    #[test]
    fn from_bytes_confirmed_data_up() {
        let mut device = create_initialized_device();
        let session_context = device.session_mut().unwrap();
        for _ in 0..13 {
            session_context.network_context_mut().f_cnt_up_autoinc();
        }

        let packet_1 = Vec::from_hex("402A3B11E0800D0003270FC620B1ADF06C1C72C21442FCAD061A91753F5C154F11DAB425056CE6156037E504C89B").unwrap(); //real packet uniorchestra

        let packet_2 = LoRaWANPacket::from_bytes(&packet_1, Some(&device), true).unwrap();

        println!("{packet_2:?}");
    }

    #[test]
    fn from_bytes_join_accept() {
        let device = create_uninitialized_device();

        let packet =
            Vec::from_hex("2076281796279C3FF432A37FAA6791C806E9278DDA0A629E149C96978F57C0FE36")
                .unwrap();


        let packet1 = LoRaWANPacket::from_bytes(&packet, Some(&device), false).unwrap();

        println!("{packet1:?}");
    }

    #[test]
    fn from_bytes_downlink_ack() {
        let mut device = create_initialized_device();

        let session_context = device.session_mut().unwrap();
        //for _ in 0..2 {
        //    session_context.network_context_mut().f_cnt_up_autoinc();
        //}

        for _ in 0..9 {
            session_context.network_context_mut().nf_cnt_dwn_autoinc();
        }

        let packet = Vec::from_hex("602A3B11E0A009000064F7AC5D").unwrap();


        let packet1 = LoRaWANPacket::from_bytes(&packet, Some(&device), false).unwrap();

        println!("{packet1:?}");
    }

    #[test]
    fn from_bytes_edcommands() {
        let _list = vec![
            EDMacCommands::ResetInd(1),
            EDMacCommands::DeviceTimeReq,
            EDMacCommands::RXParamSetupAns {
                rx1_dr_offset_ack: true,
                rx2_data_rate_ack: false,
                channel_ack: false,
            },
            EDMacCommands::DevStatusAns {
                battery: 200,
                margin: 10,
            },
            EDMacCommands::DlChannelAns {
                uplink_frequency_exists: true,
                channel_frequency_ok: true,
            },
        ];


        let payload_content = Vec::from_hex("0331FFFF6106").unwrap();

        println!("{}", PrettyHexSlice(&payload_content));


        let commands = NCMacCommands::from_bytes(&payload_content).unwrap();

        println!("{commands:?}\n\n");
    }

    #[test]
    fn generic_from_bytes() {
        
        let mut device = create_initialized_device();
        //device.set_dev_nonce(9138);
        //println!("{}", device);
        
        let bytes = Vec::from_hex("602A3B11E0B00200000E5D48517322F2568B28").unwrap();


        let packet = LoRaWANPacket::from_bytes(&bytes, Some(&device), false).unwrap();
        println!("{packet:?}");


        match packet.payload() {
            Payload::JoinRequest(_jr) => todo!(),
            Payload::JoinAccept(ja) => {
                device.derive_session_context(ja).unwrap();
                println!("{device}")
            },
            Payload::RejoinRequest(_rj) => todo!(),
            Payload::MACPayload(mp) => {
                if let (Some(port), Some(payload)) = (mp.fport(), mp.frm_payload()) {
                    if port == 0 && !payload.is_empty() {
                        let commands = NCMacCommands::from_bytes(payload).unwrap();
                        //println!("{}", PrettyHexSlice(payload));
                        println!("{commands:?}")
                    }
                }  
            },
            Payload::Proprietary(_p) => todo!(),
        }
    }

    #[test]
    fn test_fhdr_fopts_encryption() {
        let d = create_initialized_device();
        let p = [1,2,3,4,5,6];

        let mut fhdr = FHDR::new([1,2,3,4], FCtrl::Uplink(UplinkFCtrl::new(false, true, false, true, 0)));
        fhdr.set_fopts(&p);
        println!("{fhdr:?}");
        
        let bytes = fhdr.to_bytes_with_context(&d).unwrap();
        println!("{bytes:?}");
        
        let nfhdr = FHDR::from_bytes(&bytes, Some(&d), true).unwrap();
        println!("{nfhdr:?}");
    }
}
