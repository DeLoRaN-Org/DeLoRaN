use std::{time::{SystemTime, UNIX_EPOCH}, thread, fs::OpenOptions};
use std::io::Write;
use blockchain_api::{exec_bridge::{BlockchainExeConfig, BlockchainExeClient}, BlockchainClient};

fn get_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}


fn update_file(path: &str, before: u128, after: u128) {
    let mut file = OpenOptions::new()
    .append(true)
    .create(true)
    .open(path)
    .expect("Failed to open file");
    let delay = after - before;
    writeln!(file, "{},{before},{after},{delay}", SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis()).expect("Error while logging time to file");
}

async fn create_convergence_flag(client: &BlockchainExeClient) {
    let before = get_epoch_ms();
    client.create_flag().await.expect("Error creating flag");
    let after = get_epoch_ms();

    update_file("/root/writer_convergence_times.csv", before, after)
    //let creation_flag_time = after - before; //println!("Start creating flag for convergence experiment: {}", before); //println!("Stop experiment:                                {}", after); //println!( //    "Creation flag time:                             {:?}", //    creation_flag_time //); //let _end: Option<u32> = None;
}

async fn clear_convergence_flag(client: &BlockchainExeClient) {
    //let before = get_epoch_ms();
    client.clear_flag().await.expect("Error clearing flag");
    //let after = get_epoch_ms();
}

async fn read_convergence_flag(client: &BlockchainExeClient) {
    let before = get_epoch_ms();
    let mut flag = String::from("notaflag");
    while flag == "notaflag" {
        flag = client.get_flag().await.expect("Error reading flag");
    }
    let after = get_epoch_ms();
    update_file("/root/reader_convergence_times.csv", before, after)
}

async fn async_main() {
    let role = std::env::args().nth(1).expect("no command line arguments");
    let bc_config = BlockchainExeConfig {
        orderer_addr: "orderer1.orderers.dlwan.phd:6050".to_string(),
        channel_name: "lorawan".to_string(),
        chaincode_name: "lorawan".to_string(),
        orderer_ca_file_path: None,
    };
    let client = *BlockchainExeClient::from_config(&bc_config).await.unwrap();
    if role == "writer" {
        loop {
            clear_convergence_flag(&client).await;
            thread::sleep(std::time::Duration::from_secs(5));
            create_convergence_flag(&client).await;
            thread::sleep(std::time::Duration::from_secs(5));
        }
    } else if role == "reader" {
        loop {
            read_convergence_flag(&client).await;
            thread::sleep(std::time::Duration::from_secs(10));
        }
    } else {
        println!("Usage: ./a.out [writer|reader]");
    }


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
        let dc = format!(
            "{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );
        println!("{:?}", dc);
    }
}
