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
    //create_configs(0, 12);    
    //let client = BlockchainExeClient::new("orderer1.orderers.dlwan.phd:6050", "lorawan", "lorawan", None).await;
    //let state = client.get_public_blockchain_state().await.unwrap();
    //let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();
    //let path = format!("../simulation/output/state_{timestamp}.json");
    //let mut out_state = File::create(path).unwrap();
    //writeln!(out_state, "{}", serde_json::to_string_pretty(&state).unwrap()).unwrap();    
    //let dev_addr = EUI64::from([232,5,211,58,157,111,23,37]); 
    //let ans = client.get_device(&dev_addr).await.unwrap();

    //let mut c = 0;
    //match client.create(&_create_uninitialized_device()).await {
    //    Ok(v) => println!("Call {} success: {v:?}", c),
    //    Err(e) => eprintln!("Call {} failed, {e:?}", c),
    //}
    //c += 1;
    //println!();
    //match client.get_hash().await {
    //    Ok(v) => println!("Call {} success: {v:?}", c),
    //    Err(e) => eprintln!("Call {} failed, {e:?}", c),
    //}
    //c += 1;
    //println!();
    //match client.get_device(&EUI64::from([80,222,38,70,249,167,172,142])).await {
    //    Ok(v) => println!("Call {} success: {v:?}", c),
    //    Err(e) => eprintln!("Call {} failed, {e:?}", c),
    //}
    //c += 1;
    //println!();
    //match client.get_device_session(&[80,222,38,70]).await {
    //    Ok(v) => println!("Call {} success: {v:?}", c),
    //    Err(e) => eprintln!("Call {} failed, {e:?}", c),
    //}
    //c += 1;
    //println!();
    //match client.get_device_config(&EUI64::from_hex("50DE2646F9A7AC8E").unwrap()).await {
    //    Ok(v) => println!("Call {} success: {v:?}", c),
    //    Err(e) => eprintln!("Call {} failed, {e:?}", c),
    //}

    //match client.get_public_blockchain_state().await {
    //    Ok(v) => println!("Call {} success: {v:?}", c),
    //    Err(e) => eprintln!("Call {} failed, {e:?}", c),
    //}
    //c += 1;
    //println!();
    //match client.delete_device(&EUI64::from([80,222,38,70,249,167,172,142])).await {
    //    Ok(v) => println!("Call {} success: {v:?}", c),
    //    Err(e) => eprintln!("Call {} failed, {e:?}", c),
    //}
    //c += 1;
    //println!();
    ////client.delete_device_session(&[80,222,38,70]).await {
    ////client.create_join(&[1,2,3,4,5,6,7,8,9,0], &[1,2,3,4,5,6,7,8,9,0], "id_1").await {
    ////client.create_uplink(&[1,2,3,4,5,6,7,8,9,0], Some(&[1,2,3,4,5,6,7,8,9,0]), "id_1").await {
    ////client.get_packet("abcdefabdcdef").await {
//
    //match client.get_public_blockchain_state().await {
    //    Ok(v) => println!("Call {} success: {v:?}", c),
    //    Err(e) => eprintln!("Call {} failed, {e:?}", c),
    //}
    //c += 1;
    //println!();
    
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
