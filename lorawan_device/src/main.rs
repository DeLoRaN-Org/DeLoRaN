    
use std::{collections::HashMap, fs, io::Write, net::Ipv4Addr, ops::Deref, process::{Command, Stdio}, time::Duration};
use blockchain_api::BlockchainDeviceConfig;
use lorawan_device::configs::{DeviceConfig, DeviceConfigType, RadioDeviceConfig, UDPDeviceConfig};
use lorawan::{device::{Device, DeviceClass, LoRaWANVersion}, encryption::key::Key, physical_parameters::{CodeRate, DataRate, LoRaBandwidth, SpreadingFactor}, regional_parameters::region::{Region, RegionalParameters}, utils::{eui::EUI64, PrettyHexSlice}};
use serde::{Serialize, Deserialize};
use serde_json::json;



#[allow(non_snake_case)]
#[derive(Serialize)]
struct BlockchainArgs {
    Args: Vec<String>
}

fn create_command(dev_eui: &str, orderer_addr: &str, channel_name: &str, chaincode_name: &str, invoke: bool , args: BlockchainArgs, transient_data: Option<HashMap<&'static str, Vec<u8>>>) -> String {        
    let transient_string = if let Some(v) = transient_data { serde_json::to_string(&v).unwrap() } else { String::new() };
    
    let args_string = format!("\'{}\'", serde_json::to_string(&args).unwrap().trim());
    
    let mut peer_args = vec![
        "chaincode",
        { if invoke { "invoke" } else { "query" }},
        "-o", orderer_addr,
        "-C", channel_name, 
        "-n", chaincode_name,
        "-c ", &args_string,
        "--tls",
        "--cafile", "/opt/fabric/crypto/orderer-ca.crt",
    ];


    if !transient_string.is_empty() { peer_args.extend_from_slice(&["--transient", &transient_string]) }
    if invoke { peer_args.push("--waitForEvent") }
    println!("{dev_eui} -- peer {}\n", peer_args.join(" "));
    format!("peer {}", peer_args.join(" "))
}

fn create_device_command(device: &Device) -> String {
    let config: BlockchainDeviceConfig = device.into();
    let str_device = serde_json::to_string(&config).unwrap();
    let args = BlockchainArgs {
        Args: vec![
            "CreateDeviceConfig".to_owned(),
            str_device
        ],
    };

    let dev_eui = PrettyHexSlice(device.dev_eui().deref()).to_string();

    create_command(&dev_eui, "orderer1.orderers.dlwan.phd:6050", "lorawan", "lorawan", true,args, None)
}

fn create_configs(devices_to_skip: usize, num_devices: usize, devices_per_device: usize) {
    let file_content = fs::read_to_string("../simulation/devices_augmented.csv").unwrap();

    let mut commands = vec![];
    let mut devices = Vec::new();

    let mut index = 0;
    let _colosseum_address: Ipv4Addr = "192.169.40.2".parse().unwrap();

    let mut i = 0;
    file_content.split('\n').skip(devices_to_skip).take(num_devices * devices_per_device).for_each(|line| {
        let splitted = line.split(',').collect::<Vec<&str>>();
        let dev_eui = splitted[0];
        let join_eui = splitted[1];
        let key = splitted[2];
        
        let d = Device::new(DeviceClass::A, Some(RegionalParameters::new(Region::EU863_870)), EUI64::from_hex(dev_eui).unwrap(), EUI64::from_hex(join_eui).unwrap(), Key::from_hex(key).unwrap(), Key::from_hex(key).unwrap(), LoRaWANVersion::V1_0_4);
        
        let _r = RadioDeviceConfig {
            region: Region::EU863_870,
            spreading_factor: SpreadingFactor::SF7,
            data_rate: DataRate::DR5,
            bandwidth: LoRaBandwidth::BW125,
            freq: 1010000000.0,
            sample_rate: 1000000.0,
            rx_chan_id: 0,
            tx_chan_id: 1,
            code_rate: CodeRate::CR4_5,
        };
        commands.push(json!({
            "dev_eui": dev_eui,
            "command": create_device_command(&d)
        }));

        /*let config = DeviceConfig {
            configuration: d,
            dtype: DeviceConfigType::COLOSSEUM(ColosseumDeviceConfig {
                radio_config: r,
                address: colosseum_address,
                sdr_code: String::from("./src/sdr-lora-merged.py")
            }),
        };*/



        let addresses = [
            "10.207.19.155",
            "10.207.19.20",
            "10.207.19.81",
            "10.207.19.223",
            "10.207.19.66",
            "10.207.19.206",
            "10.207.19.38",
            "10.207.19.26",
            "10.207.19.94",
            "10.207.19.113",
            "10.207.19.95",
            "10.207.19.70",
            "10.207.19.71",
            "10.207.19.24",
            "10.207.19.212",
            "10.207.19.102",
        ];


        let udp_config = UDPDeviceConfig {
            addr: addresses[i % addresses.len()].to_string(),
            port: 9090,
        };

        let config = DeviceConfig {
            configuration: d,
            dtype: DeviceConfigType::UDP(udp_config),
        };

        devices.push(serde_json::to_value(config).unwrap());
        
         i+= 1;

        if devices.len() == devices_per_device {
            let path = format!("./configs/{index}_config.json");
            index += 1;

            let c = json!({
                "devices": devices
            });

            std::fs::write(path, c.to_string()).unwrap();
            devices.clear();
        }
    });
    
    std::fs::write("./configs/create_commands.json", serde_json::Value::Array(commands).to_string()).unwrap()
}


#[derive(Debug, Clone, Serialize, Deserialize)]
struct Commands {
    dev_eui: String,
    command: String
}

#[allow(dead_code)]
fn send_commands(nc_endpoints: &[&str], devices_per_device: usize) {
    let mut commands: Vec<Commands> = serde_json::from_str(&fs::read_to_string("./configs/create_commands.json").unwrap()).unwrap();
    

    for (chunk, endpoint) in commands.chunks_mut(devices_per_device).zip(nc_endpoints.iter()) {
        let mut c = Command::new("ssh").stdin(Stdio::piped()).stdout(Stdio::piped()).arg(endpoint).spawn().unwrap();
        
        let stdin = c.stdin.as_mut().unwrap();
        //let stdout = c.stdout.as_mut().unwrap();
    
        //let mut s = String::with_capacity(2048);
    
        stdin.write_all("source /root/mini_scripts/source_peer.sh\n".as_bytes()).unwrap();
    
        for c in chunk.iter_mut() {
            //println!("{} - {}",c.dev_eui, c.command);
            c.command.push('\n');
            println!("{}", c.command);
            stdin.write_all(c.command.as_bytes()).unwrap();
            //unsafe {
            //    stdout.read_exact(s.as_bytes_mut()).unwrap();
            //}
            std::thread::sleep(Duration::from_millis(500));
        }
        c.wait().unwrap();
    }
}

#[tokio::main]
async fn main() {
    #[allow(unused)]
    let nc_endpoint = ["wineslab-049"];
    #[allow(unused)]
    let devices_endpoint = ["wineslab-049"];
    let devices_per_device = 40000;
    create_configs(0, 1, devices_per_device);
    //send_commands(&nc_endpoint, devices_per_device);
}


#[cfg(test)]
mod test {
    use core::panic;

    use lorawan::{device::{session_context::{ApplicationSessionContext, NetworkSessionContext, SessionContext}, Device, DeviceClass, LoRaWANVersion}, encryption::key::Key, lorawan_packet::LoRaWANPacket, physical_parameters::{CodeRate, LoRaBandwidth, SpreadingFactor}, utils::eui::EUI64};
    use  lorawan_device::{communicator::{ArrivalStats, Position, ReceivedTransmission, Transmission}, devices::{debug_device::DebugDevice, lorawan_device::LoRaWANDevice, mock_device::MockCommunicator}};

    #[test]
    fn print_transmission() {
        let transmission = ReceivedTransmission {
            transmission: Transmission {
                start_position: Position {
                    x: 1.2,
                    y: 3.4,
                    z: 5.6,
                },
                start_time: 1234567890,
                frequency: 868_000_000.0,
                bandwidth: LoRaBandwidth::BW125,
                spreading_factor: SpreadingFactor::SF7,
                code_rate: CodeRate::CR4_5,
                starting_power: 14.0,
                uplink: true,
                payload: vec![1,2,3,4,5,6,7],
            },
            arrival_stats: ArrivalStats {
                time: 1234567890,
                rssi: -20.0,
                snr: 20.0,
            },
        };

        println!("{}", serde_json::to_string_pretty(&transmission).unwrap());
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
    


    #[tokio::test]
    async fn test() {
        let mut ld = DebugDevice::from(LoRaWANDevice::new(create_initialized_device(), MockCommunicator));         
        for _ in 0..1000 {
            let uplink = ld.create_uplink(Some("###  confirmed 5 message  ###".as_bytes()), false, Some(1), None).unwrap();
    
            match LoRaWANPacket::from_bytes(&uplink, Some(&*ld), true) {
                Ok(_) => {
                    //println!("{:?}", l)
                },
                Err(e) => {
                    println!("{:?}", e);
                    panic!("help")
                },
            };
        }
    }
}