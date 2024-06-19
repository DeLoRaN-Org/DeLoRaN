#![allow(non_snake_case, unused)]

use std::{collections::HashMap, time::{Duration, Instant, SystemTime}, fs::File, path::Path, process::Command as SyncCommand};

use lorawan_device::{communicator::LoRaWANCommunicator, configs::{TcpDeviceConfig, UDPDeviceConfig}, devices::{lorawan_device::LoRaWANDevice, udp_device::UDPDevice}};
use lorawan::{
    device::{Device, DeviceClass, LoRaWANVersion},
    encryption::key::Key,
    utils::eui::EUI64, lorawan_packet::{LoRaWANPacket, mhdr::MType, payload::Payload},
};
use paho_mqtt::Client;
use prost::Message;
use serde::{Deserialize, Serialize};
use tokio::{sync::mpsc, process::Command};
use std::io::Write;

use crate::{compiled::gw::{UplinkFrame, UplinkTxInfo, Modulation, modulation::Parameters, LoraModulationInfo, UplinkRxInfo, DownlinkFrame}, RANDOM_JOIN_DELAY, RANDOM_PACKET_DELAY, FIXED_PACKET_DELAY, NUM_PACKETS, NUM_DEVICES};

#[derive(Serialize, Deserialize, Debug)]
struct DeviceStatus {
    batteryLevel: u32,
    externalPowerSource: bool,
    margin: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct DeviceAns {
    createdAt: String,
    description: String,
    devEui: String,
    deviceProfileId: String,
    deviceProfileName: String,
    deviceStatus: Option<DeviceStatus>,
    lastSeenAt: Option<String>,
    name: String,
    updatedAt: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChirpstackListDeviceAns {
    totalCount: u32,
    result: Vec<DeviceAns>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChirpstackDeviceKeys {
    devEui: String,
    nwkKey: String,
    appKey: String, //always 0
}

#[derive(Serialize, Deserialize, Debug)]
struct ChirpstackDevice {
    deviceKeys: ChirpstackDeviceKeys,
    createdAt: String,
    updatedAt: String,
}

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
    let rtt_path = format!("./output/chirpstack_rtt_d{}_p{}_{}.csv", NUM_DEVICES, NUM_PACKETS, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis());
    let mut rtt_file = File::create(rtt_path).unwrap();

    let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();
    
    let sls_path = format!("./output/chirpstack_simulation_stats_{timestamp}.csv");

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
            r#"docker stats --no-stream | grep "chirpstack" | awk {'print $2'}"#,
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
    loop {
        //let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();

        let cmd = r#"docker stats --no-stream | grep "chirpstack" | awk '{print $2 "," $3 "," $4}'"#;
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

fn create_uplink(payload: &[u8]) -> UplinkFrame {
    UplinkFrame {
        phy_payload: payload.to_vec(),
        tx_info_legacy: None,
        rx_info_legacy: None,
        tx_info: Some(UplinkTxInfo {
            frequency: 868100000,
            modulation: Some(Modulation {
                parameters: Some(Parameters::Lora(LoraModulationInfo {
                    bandwidth: 125000,
                    spreading_factor: 7,
                    code_rate_legacy: String::new(),
                    code_rate: 1, //CR 4/5
                    polarization_inversion: false,
                })),
            }),
        }),
        rx_info: Some(UplinkRxInfo {
            gateway_id: "06b302c2f002cfa3".to_owned(),
            uplink_id: rand::random(),
            time: None,
            time_since_gps_epoch: None,
            fine_time_since_gps_epoch: None,
            rssi: 50,
            snr: 5.5,
            channel: 1,
            rf_chain: 1,
            board: 1,
            antenna: 1,
            context: vec![1,2,3,4],
            metadata: HashMap::new(),
            crc_status: 0,
            location: None,
        }),
    }

}

async fn device_routine<T: LoRaWANCommunicator + Send + Sync>(mut fd: LoRaWANDevice<T>, i: usize, sender: mpsc::Sender<Msg>, start: Instant) {
    //println!("{fd:?}");
    let client = Client::new("tcp://127.0.0.1:1883").unwrap();
    let down_topic = "eu868/gateway/06b302c2f002cfa3/command/down";
    let up_topic = "eu868/gateway/06b302c2f002cfa3/event/up";

    client.connect(None).unwrap();
    client.subscribe(down_topic, paho_mqtt::QOS_1).unwrap();
    let receiver = client.start_consuming();

    fd.set_dev_nonce(39);

    let mut sleep_time: u64 = rand::random::<u64>() % RANDOM_JOIN_DELAY;
    tokio::time::sleep(Duration::from_secs(sleep_time)).await;

    let jr = fd.create_join_request().unwrap();
    let content = create_uplink(&jr);

    let v = content.encode_to_vec();
    //println!("{i}: {}", PrettyHexSlice(&v));

    let mut before = Instant::now();

    client
        .publish(paho_mqtt::Message::new(up_topic, &*v, paho_mqtt::QOS_1))
        .unwrap();

    let mut counter_uplink = 0_usize;

    while let Ok(Some(msg)) = receiver.recv() {
        let dwn = DownlinkFrame::decode(msg.payload()).unwrap();
        
        //println!("dwn.items len{}", dwn.items.len());

        if dwn.items.is_empty() {
            println!("{i}: {dwn:?}")
        } else {
            let downlink = &dwn.items[0];    
            let payload = &downlink.phy_payload;
            
            //println!("{downlink:?}");
            //println!("{}", PrettyHexSlice(payload));
            //if msgs.contains(&PrettyHexSlice(payload).to_string()) {
            //    continue;
            //} else {
            //    msgs.insert(PrettyHexSlice(payload).to_string());
            //}
            
            let mtype = LoRaWANPacket::extract_mtype(payload[0]);
            match mtype {
                MType::JoinAccept => {
                    //println!("{i}: Received join_accept: {}", PrettyHexSlice(payload));
                    match LoRaWANPacket::from_bytes(payload, Some(&fd), false) {
                        Err(err) => {
                            //eprintln!("{i}: {err:?}");
                            continue;
                        },
                        Ok(decrypted) => {
                            sender.send(Msg { thread_id: i, stats: Stats::Rtt, content: format!("{}, {}",before.elapsed().as_millis(), start.elapsed().as_millis()) }).await.unwrap();

                            let decryped = LoRaWANPacket::from_bytes(payload, Some(&*fd), false).unwrap().into_payload();
                            if let Payload::JoinAccept(ja) = decryped {
                                fd.generate_session_context(&ja).unwrap();
        
                                let payload: Vec<u8> = format!("### confirmed {i} message  ###").into();
                                let uplink = fd.create_uplink(Some(&payload), true, Some(1), None).unwrap(); 
                                let uplink = create_uplink(&uplink).encode_to_vec();
        
                                sleep_time = rand::random::<u64>() % RANDOM_PACKET_DELAY;
                                tokio::time::sleep(Duration::from_secs(FIXED_PACKET_DELAY + sleep_time)).await;
        
                                before = Instant::now();
                                client.publish(paho_mqtt::Message::new(up_topic, &*uplink, paho_mqtt::QOS_1)).unwrap();
                                counter_uplink += 1;
                                //println!("published uplink: {}", PrettyHexSlice(&uplink));
                            }
                        },
                    }
                },
                MType::UnconfirmedDataDown => {
                    //println!("{i}: Received unconfirmed data down: {}", PrettyHexSlice(payload));
                    match LoRaWANPacket::from_bytes(payload, Some(&*fd), false) {
                        Err(err) => {
                            //eprintln!("{i}: {err:?}");
                            continue;
                        },
                        Ok(decrypted) => {
                            sender.send(Msg { thread_id: i, stats: Stats::Rtt, content: format!("{}, {}",before.elapsed().as_millis(), start.elapsed().as_millis()) }).await.unwrap();
                            if let Payload::MACPayload(dd) = decrypted.into_payload() {
                                let payload: Vec<u8> = format!("ProvaProvaProva {i}").into();
                                let uplink_vec = fd.create_uplink(Some(&payload), true, Some(1), None).unwrap();
                                let uplink = create_uplink(&uplink_vec).encode_to_vec();
        
                                sleep_time = rand::random::<u64>() % RANDOM_PACKET_DELAY;
                                tokio::time::sleep(Duration::from_secs(FIXED_PACKET_DELAY + sleep_time)).await;
                                
                                before = Instant::now();
                                client.publish(paho_mqtt::Message::new(up_topic, &*uplink, paho_mqtt::QOS_1)).unwrap();
                                counter_uplink += 1;
                                //println!("published uplink: {}", PrettyHexSlice(&uplink_vec));
                            }
                        },
                    }
                },
                MType::ConfirmedDataDown => {
                    //println!("{i}: Received confirmed data down: {}", PrettyHexSlice(payload));
                    match LoRaWANPacket::from_bytes(payload, Some(&*fd), false) {
                        Err(err) => {
                            //eprintln!("{i}: {err:?}");
                            continue;
                        },
                        Ok(decrypted) => {
                            sender.send(Msg { thread_id: i, stats: Stats::Rtt, content: format!("{}, {}",before.elapsed().as_millis(), start.elapsed().as_millis()) }).await.unwrap();
                            if let Payload::MACPayload(dd) = decrypted.into_payload() {
                                let payload: Vec<u8> = format!("ProvaProvaProva {i}").into();
                                let uplink = fd.create_uplink(Some(&payload), true, Some(1), None).unwrap();
                                let uplink = create_uplink(&uplink).encode_to_vec();
        
                                sleep_time = rand::random::<u64>() % RANDOM_PACKET_DELAY;
                                tokio::time::sleep(Duration::from_secs(FIXED_PACKET_DELAY + sleep_time)).await;
                                
                                before = Instant::now();
                                client.publish(paho_mqtt::Message::new(up_topic, &*uplink, paho_mqtt::QOS_1)).unwrap();
                                counter_uplink += 1;
                                //println!("published uplink: {}", PrettyHexSlice(&uplink));
                            }
                        },
                    }
                },
                
                MType::UnconfirmedDataUp |
                MType::ConfirmedDataUp |
                MType::RejoinRequest |
                MType::JoinRequest |
                MType::Proprietary => unreachable!("No uplinks here, this is wrong"),
            }
        
        }
        if counter_uplink >= NUM_PACKETS {
            break;
        }

        println!("Task{i} published {counter_uplink} uplinks");
        //println!("Task {i} waiting for messages...");
    }

    println!("Task {i} closing...");
}

pub async fn main_chirpstack() {
    let client = reqwest::Client::new();

    let start = Instant::now();

    let (sender, receiver) = mpsc::channel::<Msg>(NUM_DEVICES);

    tokio::spawn(async move {
        stats_holder(receiver).await
    });
    
    let s1 = sender.clone();
    tokio::spawn(async move {
        network_live_stats_loop(s1).await
    });

    let ans = client.get("http://127.0.0.1:8090/api/devices")
        .query(&[
            //("applicationId","17272d19-e169-49a4-82e7-fa8ae17439ad"),
            ("applicationId","b8d129fe-1d37-4944-beed-ad500e00aa95"),
            ("limit","400"),
        ])
        .header("Accept", "application/json")
        .header("Grpc-Metadata-Authorization", "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJhdWQiOiJjaGlycHN0YWNrIiwiaXNzIjoiY2hpcnBzdGFjayIsInN1YiI6ImFlYzhmNzE5LWI0Y2MtNDNhYi05ZWEyLWQ0YWZmYWY3MzNlYSIsInR5cCI6ImtleSJ9.Rx5-zIhjZSeUPCEqFfZkjll7acfjc-4cyFOLPnrNPS8")
        //.header("Grpc-Metadata-Authorization", "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJhdWQiOiJjaGlycHN0YWNrIiwiaXNzIjoiY2hpcnBzdGFjayIsInN1YiI6IjE0NzA3ZWQ2LTU4YzYtNDdkMS04OWQ0LTgzNjRiMjkzMDllYSIsInR5cCI6ImtleSJ9.B7m6iadZRCfr5mH7v5V1ig79GVr8X8Aw7RpboovZ7ow")
        //.header("applicationId", "52f14cd4-c6f1-4fbd-8f87-4025e1d49242")
        //.header("tenantId", "917c0850-0ae2-4a1b-b716-f6df37bb732b")
        .send().await.unwrap();

    let  ans = &ans.text().await.unwrap();
    println!("{}", ans);
    let content: ChirpstackListDeviceAns = serde_json::from_str(ans).unwrap();
    
    let mut lorawan_devices : Vec<LoRaWANDevice<_>> = Vec::new();

    for (i, d )in content.result.iter().enumerate() {
        let ans = client.get(format!("http://127.0.0.1:8090/api/devices/{}/keys", d.devEui))
        .header("Accept", "application/json")
        .header("Grpc-Metadata-Authorization", "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJhdWQiOiJjaGlycHN0YWNrIiwiaXNzIjoiY2hpcnBzdGFjayIsInN1YiI6ImFlYzhmNzE5LWI0Y2MtNDNhYi05ZWEyLWQ0YWZmYWY3MzNlYSIsInR5cCI6ImtleSJ9.Rx5-zIhjZSeUPCEqFfZkjll7acfjc-4cyFOLPnrNPS8")
        .send().await.unwrap();

        let text = ans.text().await.unwrap();
        let device: ChirpstackDevice = serde_json::from_str(&text).unwrap();
        //println!("{device:#?}");

        let device: Device = Device::new(
            DeviceClass::A,
            None,
            EUI64::from_hex(&device.deviceKeys.devEui).unwrap(),
            EUI64::default(),
            Key::from_hex(&device.deviceKeys.nwkKey).unwrap(),
            Key::from_hex(&device.deviceKeys.appKey).unwrap(),
            LoRaWANVersion::V1_0_3,
        );
        let fd = UDPDevice::create(device, &UDPDeviceConfig {
            addr: "localhost".to_owned(),
            port: 9999,
        }).await;
        lorawan_devices.push(fd);
        println!("Got {i} device");
    }

    let mut handles = Vec::new();
    
    for (i, mut fd) in lorawan_devices.into_iter().enumerate().take(NUM_DEVICES) {
        let cloned_sender = sender.clone();
        handles.push(tokio::spawn(async move { device_routine(fd,i,cloned_sender, start).await }));
    }

    for handle in handles {
        handle.await;
    }
    
}
