use blockchain_api::{BlockchainDeviceConfig, exec_bridge::BlockchainAns};
use lorawan::{
    device::{
        session_context::{ApplicationSessionContext, NetworkSessionContext, SessionContext},
        Device, DeviceClass, LoRaWANVersion,
    },
    encryption::key::Key,
    utils::eui::EUI64, regional_parameters::region::{RegionalParameters, Region},
};

fn _create_initialized_device() -> Device {
    let mut device = Device::new(
        DeviceClass::A,
        None,
        EUI64::from_hex("50DE2646F9A7AC8E").unwrap(),
        EUI64::from_hex("DCBC65F607A47DEA").unwrap(),
        Key::from_hex("BBF326BE9AC051453AA616410F110EE7").unwrap(),
        Key::from_hex("BBF326BE9AC051453AA616410F110EE7").unwrap(),
        LoRaWANVersion::V1_0_3,
    );

    let network_context = NetworkSessionContext::new(
        Key::from_hex("32767A6BC1596EB45CA89B5910B4774E").unwrap(),
        Key::from_hex("32767A6BC1596EB45CA89B5910B4774E").unwrap(),
        Key::from_hex("32767A6BC1596EB45CA89B5910B4774E").unwrap(),
        [0x60, 0x00, 0x08],
        [0xe0, 0x10, 0x41, 0x00],
        0,
        0,
        0,
    );

    let application_context = ApplicationSessionContext::new(
        Key::from_hex("60D2DB6D7F3AAF168FE8F0775DB1FB91").unwrap(),
        0,
    );

    device.set_activation_abp(SessionContext::new(application_context, network_context));
    device
}

fn _create_uninitialized_device() -> Device {
    Device::new(
        DeviceClass::A,
        Some(RegionalParameters::new(Region::EU863_870)),
        EUI64::from_hex("50DE2646F9A7AC8E").unwrap(),
        EUI64::from_hex("DCBC65F607A47DEA").unwrap(),
        Key::from_hex("BBF326BE9AC051453AA616410F110EE7").unwrap(),
        Key::from_hex("BBF326BE9AC051453AA616410F110EE7").unwrap(),
        LoRaWANVersion::V1_0_3,
    )
}

async fn async_main() {
    let b = r#"{"content":{"activation_mode":"OTAA","app_key":[74,239,119,51,36,170,223,62,36,191,49,203,205,163,119,160],"class":"A","dev_addr":null,"dev_eui":[229,46,176,41,160,23,108,83],"dev_nonce":0,"join_eui":[10,212,98,74,55,147,214,94],"join_nonce":0,"js_enc_key":[209,9,95,201,208,91,155,200,167,110,67,216,63,47,112,199],"js_int_key":[100,173,119,201,244,115,111,12,178,205,59,53,148,35,140,168],"last_join_request_received":"JoinRequest","nwk_key":[74,239,119,51,36,170,223,62,36,191,49,203,205,163,119,160],"owner":"Org1MSP","region":"EU863_870","rj_count1":0,"version":"V1_0_4"}}"#;
    let a = serde_json::from_str::<BlockchainAns<BlockchainDeviceConfig>>(b).unwrap();
    println!("{a:?}");

}




fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main());
}

    //client.get_public_blockchain_state().await;
    //let ans = client.create_device_config(&_create_uninitialized_device()).await;

    //for p in state.packets {
    //    println!("{:?}", p);
    //}

    /*
    let state = client.get_all_devices().await.unwrap();
    println!("Getting devices configurations: ");
    
    for p in state.configs.iter() {
        let dev_eui = p.dev_eui;
        println!("Device configuration for: {dev_eui}");
        let config = client.get_device_config(&dev_eui).await.unwrap();
        println!("{config:?}");
    }
    
    println!("\n###############################################################\n");
    
    println!("Getting devices sessions: ");
    for p in state.sessions.iter() {
        let dev_addr = p.dev_addr;
        println!("Device session for: {}", PrettyHexSlice(&dev_addr));
        let session = client.get_device_session(&dev_addr).await.unwrap();
        println!("{session:?}");
    }
    
    println!("\n###############################################################\n");
    
    println!("Getting packets: ");
    for p in state.packets.iter() {
        println!("Packet hash: {}", p.hash);
        let session = client.get_packet(&p.hash).await.unwrap();
        println!("{session:?}");
    }

    println!("\n#########################     END     #########################\n");*/

#[cfg(test)]
mod test {
    use std::time::SystemTime;
    
    #[test]
    fn test() {
        let dc = format!("{}",SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis());
        println!("{:?}", dc);
    }
}
