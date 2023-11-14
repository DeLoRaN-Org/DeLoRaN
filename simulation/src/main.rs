#![allow(non_snake_case, unreachable_code, unused, clippy::iter_skip_zero)]
mod chirpstack;
mod compiled;

use std::{time::{Duration, SystemTime}, fs::{self, File}, net::Ipv4Addr, path::Path, process::Command as SyncCommand, str::FromStr};
use blockchain_api::{BlockchainClient, exec_bridge::BlockchainExeClient};
use chirpstack::main_chirpstack;
use lorawan_device::{tcp_device::TcpDevice, configs::TcpDeviceConfig};
use lorawan::{utils::{eui::EUI64, PrettyHexSlice}, device::{Device, DeviceClass, LoRaWANVersion}, regional_parameters::region::{Region, RegionalParameters}, encryption::key::Key};
use tokio::{net::TcpStream, time::Instant, process::Command, sync::mpsc};
use std::io::Write;

const NUM_DEVICES: usize = 200;
const NUM_PACKETS: usize = 100;
const RANDOM_JOIN_DELAY:   u64 = 120;
const FIXED_PACKET_DELAY: u64 = 60;
const RANDOM_PACKET_DELAY: u64 = 30;
const CONFIRMED_AVERAGE_SEND: u8 = 10;
const DEVICES_TO_SKIP: usize = 0;
const JUST_CREATE_DEVICE: bool = false;

#[derive(Debug)]
enum Stats {
    Rtt,
    Usage,
}

#[derive(Debug)]
struct Msg {
    stats: Stats,
    thread_id: usize,
    content: String,
}

impl Msg {
    pub fn into_csv(self) -> String {
        match self.stats {
            Stats::Rtt => format!("{},{}", self.thread_id, self.content),
            Stats::Usage => self.content,
        }
    }
}


async fn stats_holder(mut receiver: mpsc::Receiver<Msg>) {
    let rtt_path = format!("./output/rtt_d{}_p{}_{}.csv", NUM_DEVICES, NUM_PACKETS, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis());
    let mut rtt_file = File::create(rtt_path).unwrap();

    let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();
    
    let sls_path = format!("./output/simulation_stats_{timestamp}.csv");

    let usage_live_stats_path = Path::new(&sls_path);
    let mut usage_stats_file = File::create(usage_live_stats_path).unwrap();
    writeln!(usage_stats_file, "timestamp,name,cpu,ram,n_i_diff,n_o_diff").unwrap();
    
    writeln!(rtt_file, "thread_id,rtt,tmst").unwrap();

    while let Some(msg) = receiver.recv().await {
        match msg.stats {
            Stats::Rtt => writeln!(rtt_file, "{}", msg.into_csv()).unwrap(),
            Stats::Usage => writeln!(usage_stats_file, "{}", msg.into_csv()).unwrap(),
        }
    }

    println!("Channel closed, quitting stats_holder");
}

async fn create_all_devices() {
    let file_content = fs::read_to_string("./devices_augmented.csv").unwrap();
    let client = BlockchainExeClient::new("orderer1.orderers.dlwan.phd:6050", "lorawan", "lorawan", None);
    let mut join_handlers = Vec::new();


    file_content.split('\n').skip(DEVICES_TO_SKIP).take(NUM_DEVICES).for_each(|line| {
        let splitted = line.split(',').collect::<Vec<&str>>();
        let dev_eui = splitted[0];
        let join_eui = splitted[1];
        let key = splitted[2];

        //println!("{}", dev_eui);
        let d = Device::new(DeviceClass::A, Some(RegionalParameters::new(Region::EU863_870)), EUI64::from_hex(dev_eui).unwrap(), EUI64::from_hex(join_eui).unwrap(), Key::from_hex(key).unwrap(), Key::from_hex(key).unwrap(), LoRaWANVersion::V1_0_4);
        let cloned = client.clone();
        std::thread::sleep(Duration::from_millis(50));
        let handle =  tokio::spawn(async move {
            match cloned.get_device(d.dev_eui()).await {
                Ok(d) => println!("Device already exists"),
                Err(e) => {
                    match cloned.create_device_config(&d).await {
                        Ok(_) => println!("Device {} created successfully", PrettyHexSlice(&**d.dev_eui())),
                        Err(e) => println!("Failed to create device config: {e:?}"),
                    }
                },
            }
        });
        join_handlers.push(handle);
    });

    for handle in join_handlers {
        handle.await.unwrap();
    }
}

async fn network_live_stats_loop(sender: mpsc::Sender<Msg>) {
    let output = Command::new("sh")
        .args([
            "-c",
            r#"docker stats --no-stream | grep "example.com" | awk {'print $2'}"#,
        ])
        .output().await
        .unwrap();

    let output_str = String::from_utf8_lossy(&output.stdout);
    let entities_names = output_str.trim()
        .split('\n').map(|line| {
            line.trim()
        }).collect::<Vec<&str>>();
    
    let num_entities = entities_names.len();

    let mut last_input = vec![0_usize; num_entities];
    let mut last_output = vec![0_usize; num_entities];

    let usage_lock_path = Path::new("./output/simulation.lock");

    let start = Instant::now();

    while usage_lock_path.exists() {
        //let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();

        let cmd = r#"docker stats --no-stream | grep "example.com" | awk '{print $2 "," $3 "," $4}'"#;
        let output = Command::new("sh")
                    .args(["-c", cmd])
                    .output().await
                    .unwrap();
        let lines = String::from_utf8_lossy(&output.stdout);
        let trimmed = lines.trim();

        let content = trimmed.split('\n').enumerate().map(|(i,line)| {
            let ncr_vec = line.split(',').collect::<Vec<&str>>();
            let name = ncr_vec[0];
            let cpu = ncr_vec[1];
            let ram = ncr_vec[2];
            
            let command = format!("docker exec {name} sh -c 'cat /proc/net/dev' | grep eth0 | awk {{'print $2 \",\" $10'}}");
            let output = SyncCommand::new("sh")
                    .args(["-c", command.as_str()])
                    .output()
                    .unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            let in_out = stdout.trim().split(',').map(|v| v.parse().unwrap()).collect::<Vec<usize>>();
            let ni = in_out[0];
            let no = in_out[1];
            
            let n_i_diff = ni - last_input[i];
            let n_o_diff = no - last_output[i];
            last_input[i] = ni;
            last_output[i] = no;

            format!("{},{name},{cpu},{ram},{n_i_diff},{n_o_diff}", start.elapsed().as_millis())
        }).collect::<Vec<String>>();
        
        for line in content {
            sender.send(Msg {
                stats: Stats::Usage,
                thread_id: 0,
                content: line 
            }).await.unwrap();
        }
    }
}

async fn blockchain_main() {
    if JUST_CREATE_DEVICE {
        create_all_devices().await;
        return;
    }
    //let _args = Args::parse();
    //let path = Path::new("./output/simulation.lock");
    //File::create(path).unwrap();
    

    let file_content = fs::read_to_string("./devices_augmented.csv").unwrap();

    let mut join_handlers = Vec::new();

    let (sender, receiver) = mpsc::channel::<Msg>(NUM_PACKETS);


    tokio::spawn(async move {
        stats_holder(receiver).await
    });
    
    //let s1 = sender.clone();
    //tokio::spawn(async move {
    //    network_live_stats_loop(s1).await
    //});

    let start = Instant::now();


    let nc_ips = [
        "127.0.0.1".to_owned(),
        
        //"172.18.2.137".to_owned(),
        //"172.18.2.139".to_owned()
    ];

    let nc_ips_len = nc_ips.len();

    file_content.split('\n').skip(DEVICES_TO_SKIP).take(NUM_DEVICES).enumerate().for_each(|(i, line)| {
        //println!("{i}: {line}");
        let splitted = line.split(',').collect::<Vec<&str>>();
        let dev_eui = EUI64::from_hex(splitted[0]).unwrap();
        let join_eui = EUI64::from_hex(splitted[1]).unwrap();
        let key = Key::from_hex(splitted[2]).unwrap();
        
        let sender_cloned = sender.clone();
        let nc_ip = nc_ips[i % nc_ips_len].clone();

        let handle = tokio::spawn(async move {
            let thread_id = i;
            let mut sleep_time: u64 = rand::random::<u64>() % RANDOM_JOIN_DELAY;
            tokio::time::sleep(Duration::from_secs(sleep_time)).await;

            let d = Device::new(DeviceClass::A, Some(RegionalParameters::new(Region::EU863_870)), dev_eui, join_eui, key, key, LoRaWANVersion::V1_0_4);
            println!("before creating");
            let mut device = TcpDevice::create(d, &TcpDeviceConfig {
                addr: nc_ip,
                port: 9090,
            }).await;
            println!("after creating");

            if let Some(_s) = device.session() {
                println!("Device already initialized:");
                //println!("{s}");
            } else {
                println!(
                    "Device {} needs initialization, sending join request...",
                    device.dev_eui()
                );
                //let duration = 10;
                //sleep(Duration::from_secs(duration)).await;
                //device.set_dev_nonce(10);
                while let Err(e) = device.send_join_request().await {
                    panic!("Error joining: {e:?}");
                };
                println!("{}", *device);
            }

            if device.session().is_some() {
                println!("Device {dev_eui} already initialized: {}", serde_json::to_string(&*device).unwrap());
            } else {
                println!("Device needs initialization, sending join request for {dev_eui}...");
                device.send_join_request().await.unwrap();
                println!("Initialized: {}", /*serde_json::to_string(&*device).unwrap()*/ PrettyHexSlice(device.session().unwrap().network_context().dev_addr()));
            }

            //device.session_mut().unwrap().application_context_mut().update_af_cnt_dwn(10);            
            for i in 0..NUM_PACKETS {
                let sleep_time = rand::random::<u64>() % RANDOM_PACKET_DELAY;
                //device.send_uplink(Some(format!("@@@ unconfirmed {i} message @@@").as_bytes()), false, Some(1), None).await.unwrap();
                tokio::time::sleep(Duration::from_secs(FIXED_PACKET_DELAY + sleep_time)).await;
                let before = Instant::now();
                                
                //let confirmed = rand::random::<u8>() % CONFIRMED_AVERAGE_SEND == 0;
                //let confirmed = true;
                //let (un, confirmed) = if confirmed { //in media 1 su N è confirmed
                //    ("", true)
                //} else {
                //    ("un", false)
                //};

                let (un, confirmed) = ("", true);

                device.send_uplink(Some(format!("###  {un}confirmed {i} message  ###").as_bytes()), confirmed, Some(1), None).await.unwrap();
                sender_cloned.send(Msg { thread_id, stats: Stats::Rtt, content: format!("{}, {}",before.elapsed().as_millis(), start.elapsed().as_millis()) }).await.unwrap();
            }
            println!("Task {thread_id} completed successfully");
        });
        join_handlers.push(handle);
    });
    
    for handle in join_handlers {
        handle.await.unwrap();
    }
}

fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(blockchain_main())
}
