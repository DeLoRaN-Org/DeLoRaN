#![allow(non_snake_case, unreachable_code, clippy::iter_skip_zero)]
mod chirpstack;
mod compiled;

use std::{fs::{self, File, OpenOptions}, io::BufReader, time::{Duration, SystemTime}};
use blockchain_api::{BlockchainClient, exec_bridge::BlockchainExeClient};

use lorawan_device::{configs::{DeviceConfig, TcpDeviceConfig}, devices::{debug_device::DebugDevice, tcp_device::TcpDevice}};
use lorawan::{utils::{eui::EUI64, PrettyHexSlice}, device::{Device, DeviceClass, LoRaWANVersion}, regional_parameters::region::{Region, RegionalParameters}, encryption::key::Key};
use serde::Deserialize;
use tokio::{task::JoinHandle, time::Instant};
use std::io::Write;

const NUM_DEVICES: usize = 8000;
const NUM_PACKETS: usize = 100;
const RANDOM_JOIN_DELAY:   u64 = 18000;
const FIXED_JOIN_DELAY: u64 = 600;
const FIXED_PACKET_DELAY: u64 = 600;
const RANDOM_PACKET_DELAY: u64 = 17400;
const _CONFIRMED_AVERAGE_SEND: u8 = 10;
const DEVICES_TO_SKIP: usize = 0;
const JUST_CREATE_DEVICE: bool = false;
const STARTING_DEV_NONCE: u32 = 30;



#[derive(Deserialize)]
struct DevicesFile {
    devices: Vec<DeviceConfig>
}

async fn create_all_devices() {
    let content: DevicesFile = serde_json::from_reader(BufReader::new(File::open("./devices.json").unwrap())).unwrap();//fs::read_to_string("./devices.json").unwrap();
    let client = BlockchainExeClient::new("orderer1.orderers.dlwan.phd:6050", "lorawan", "lorawan", None);
    let mut join_handlers: Vec<JoinHandle<()>> = Vec::new();

    for (i, d) in content.devices.iter().map(|e| e.configuration).enumerate() {
        let cloned = client.clone();
        std::thread::sleep(Duration::from_millis(100));

        if join_handlers.len() > 100 {
            for handle in join_handlers {
                handle.await.unwrap();
            }
            join_handlers = Vec::new();
        }
        join_handlers.push(tokio::spawn(async move {
            match cloned.get_device(d.dev_eui()).await {
                Ok(d) => println!("Device {} already exists", d.dev_eui()),
                Err(_e) => {
                    match cloned.create_device_config(&d).await {
                        Ok(_) => println!("[{i}] Device {} created successfully", PrettyHexSlice(&**d.dev_eui())),
                        Err(e) => println!("Failed to create device config: {e:?}"),
                    }
                },
            }
        }));
    }

    for handle in join_handlers {
        handle.await.unwrap();
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

    //let (sender, receiver) = mpsc::channel::<Msg>(NUM_PACKETS);


    //tokio::spawn(async move {
    //    stats_holder(receiver).await
    //});


    let nc_ips = [
        "10.207.19.155",
        "10.207.19.20",
        "10.207.19.81",
        "10.207.19.223",
        //"10.207.19.66",
        //"10.207.19.206",
        //"10.207.19.38",
        //"10.207.19.26",
        //"10.207.19.94",
        //"10.207.19.113",
        //"10.207.19.95",
        //"10.207.19.70",
        //"10.207.19.71",
        //"10.207.19.24",
        //"10.207.19.212",
        //"10.207.19.102",
    ].map(|a| a.to_string());

    let nc_ips_len = nc_ips.len();

    file_content.split('\n').skip(DEVICES_TO_SKIP).take(NUM_DEVICES).enumerate().for_each(|(i, line)| {
        let splitted = line.split(',').collect::<Vec<&str>>();
        let dev_eui = EUI64::from_hex(splitted[0]).unwrap();
        let join_eui = EUI64::from_hex(splitted[1]).unwrap();
        let key = Key::from_hex(splitted[2]).unwrap();
        
        let nc_ip = nc_ips[i % nc_ips_len].clone();


        let handle = tokio::spawn(async move {
            let thread_id = i;
            let d = Device::new(DeviceClass::A, Some(RegionalParameters::new(Region::EU863_870)), dev_eui, join_eui, key, key, LoRaWANVersion::V1_0_4);
            let mut device = DebugDevice::from(TcpDevice::create(d, &TcpDeviceConfig {
                addr: nc_ip,
                port: 9090,
            }).await);

            device.set_dev_nonce(STARTING_DEV_NONCE);

            let mut sleep_time: u64 = rand::random::<u64>() % RANDOM_JOIN_DELAY;
            for _ in 0..NUM_PACKETS {
                sleep_time = rand::random::<u64>() % RANDOM_JOIN_DELAY;
                tokio::time::sleep(Duration::from_secs(FIXED_JOIN_DELAY + sleep_time)).await;
         
                //if let Some(_s) = device.session() {
                //    println!("Device already initialized:");
                //} else {
                //    println!(
                //        "Device {} needs initialization, sending join request...",
                //        device.dev_eui()
                //    );
                    if let Err(e) = device.send_join_request().await {
                        panic!("Error joining: {e:?}");
                    };
                    println!("Initialized: {}", /*serde_json::to_string(&*device).unwrap()*/ PrettyHexSlice(device.session().unwrap().network_context().dev_addr()));
                //}
            }

            tokio::time::sleep(Duration::from_secs(FIXED_JOIN_DELAY + RANDOM_JOIN_DELAY - sleep_time)).await;
            //device.session_mut().unwrap().application_context_mut().update_af_cnt_dwn(10);            
            
            for i in 0..NUM_PACKETS {
                let sleep_time = rand::random::<u64>() % RANDOM_PACKET_DELAY;
                tokio::time::sleep(Duration::from_secs(FIXED_PACKET_DELAY + sleep_time)).await;
                let before = Instant::now();                
                
                let confirmed = true;
                device.send_uplink(Some(format!("###  {}confirmed {i} message  ###", if confirmed {"un"} else {""}).as_bytes()), confirmed, Some(1), None).await.unwrap();
                let rtt = before.elapsed().as_millis();
                println!("Device {} sent and received {i}-th message", dev_eui);

                if true {
                    let mut file = OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open("/root/rtt_response_times.csv")
                    .expect("Failed to open file");
                    writeln!(file, "{},{}", SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis(), rtt).expect("Error while logging time to file");
                }
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


/*
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

*/