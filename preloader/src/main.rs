pub mod device;

use std::{
    fs::File,
    io::{BufReader, Read},
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener},
};

use application_server::application_server::{ApplicationServer, ApplicationServerConfig};
use clap::Parser;
use consensus::{consensus_server::ConsensusConfig, ConsensusCerts};
use lorawan_device::{configs::{ColosseumDeviceConfig, DeviceConfig, DeviceConfigType, RadioDeviceConfig, UDPNCConfig}, devices::radio_device::RadioCommunicator};
use lazy_static::lazy_static;
use lorawan::{
    device::{
        session_context::{ApplicationSessionContext, NetworkSessionContext, SessionContext},
        Device, DeviceClass, LoRaWANVersion,
    },
    encryption::key::Key,
    physical_parameters::{CodeRate, DataRate, LoRaBandwidth, SpreadingFactor},
    regional_parameters::region::Region,
    utils::eui::EUI64,
};
use network_controller::modules::network_controller::NetworkController;
use serde::{Deserialize, Serialize};
use blockchain_api::udp_bridge::{BlockchainUDPClient, BlockchainUDPConfig};

use crate::device::device_main;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
///Preloader for DeLoRaN
struct Args {
    /// Path of the configuration JSON file.
    #[clap(short, long, value_parser)]
    config: Option<String>,
    
    #[clap(short, long, value_parser)]
    pcode: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct NetworkControllerConfig {
    pub nc_id: String,
    pub orderer_address: String,
    udp_config: Option<UDPNCConfig>,
    radio_config: Option<RadioDeviceConfig>,
    colosseum_config: Option<ColosseumDeviceConfig>,
    consensus_config: ConsensusConfig,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub devices: Option<Vec<DeviceConfig>>,
    pub device: Option<DeviceConfig>,
    pub network_controller: Option<NetworkControllerConfig>,
    pub application_server: Option<ApplicationServerConfig>,
}

impl Config {
    pub fn into_configs(
        self,
    ) -> (
        Option<Vec<DeviceConfig>>,
        Option<DeviceConfig>,
        Option<NetworkControllerConfig>,
        Option<ApplicationServerConfig>,
    ) {
        (
            self.devices,
            self.device,
            self.network_controller,
            self.application_server,
        )
    }
}

async fn network_controller_main(config: &'static NetworkControllerConfig) {
    let nc = NetworkController::new(
        config.nc_id.as_ref(),
        config.consensus_config.clone()
    );

    lazy_static!(
        static ref CONFIG: Config = serde_json::from_reader::<BufReader<File>, Config>(BufReader::new(
            File::open(Args::parse().config.unwrap()).unwrap(),
        ))
        .unwrap();


        //static ref BC_CONFIG: BlockchainExeConfig = BlockchainExeConfig {
        //    orderer_addr: CONFIG.network_controller.as_ref().unwrap().orderer_address.clone(),
        //    channel_name: "lorawan".to_string(),
        //    chaincode_name: "lorawan".to_string(),
        //    orderer_ca_file_path: None,
        //};
        
        static ref BC_CONFIG: BlockchainUDPConfig = BlockchainUDPConfig {
            port: 9999
        };
    );

    //let t1 = config.colosseum_config.as_ref().map(|colosseum_config| nc.routine::<ColosseumCommunicator, BlockchainUDPClient>(colosseum_config, &BC_CONFIG));
    let t2 = config.radio_config.as_ref().map(|radio_config| nc.routine::<RadioCommunicator, BlockchainUDPClient>(radio_config, &BC_CONFIG));
    //let t3 = config.tcp_config.as_ref().map(|tcp_config| nc.tcp_routine::<BlockchainUDPClient>(tcp_config, &BC_CONFIG));
    let t3 = config.udp_config.as_ref().map(|udp_config| nc.udp_routine::<BlockchainUDPClient>(udp_config, &BC_CONFIG));

    //if let Some(t) = t1 { t.await.unwrap(); }
    if let Some(t) = t2 { t.await.unwrap(); }
    if let Some(t) = t3 { t.await.unwrap(); }
}

async fn application_server_main(config: &'static ApplicationServerConfig) {
    let application_server = ApplicationServer::init(config).await;
    application_server.routine().await.unwrap();
}

pub fn create_initialized_device() -> Device {
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

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), std::io::Error> {
    let _config = Config {
        devices: None,
        device: Some(DeviceConfig {
            dtype: DeviceConfigType::TCP(lorawan_device::configs::TcpDeviceConfig {
                addr: "127.0.0.1".to_owned(),
                port: 9090,
            }),
            configuration: create_initialized_device(),
        }),
        network_controller: Some(NetworkControllerConfig {
            nc_id: "ns_test_1".to_string(),
            orderer_address: "orderer1.orderers.dlwan.phd".to_string(),
            udp_config: Some(UDPNCConfig { 
                addr: "0.0.0.0".to_string(), 
                port: 9090 
            }),
            radio_config: Some(RadioDeviceConfig {
                region: Region::EU863_870,
                spreading_factor: SpreadingFactor::SF7,
                data_rate: DataRate::DR5,
                bandwidth: LoRaBandwidth::BW125,
                sample_rate: 1_000_000.0,
                freq: 990_000_000.0,
                rx_chan_id: 0,
                tx_chan_id: 1,
                code_rate: CodeRate::CR4_5
            }),
            colosseum_config: Some(ColosseumDeviceConfig {
                radio_config: RadioDeviceConfig {
                    region: Region::EU863_870,
                    spreading_factor: SpreadingFactor::SF7,
                    data_rate: DataRate::DR5,
                    bandwidth: LoRaBandwidth::BW125,
                    sample_rate: 1_000_000.0,
                    freq: 990_000_000.0,
                    rx_chan_id: 0,
                    tx_chan_id: 1,
                    code_rate: CodeRate::CR4_5
                },
                address: IpAddr::V4(Ipv4Addr::LOCALHOST),
                sdr_code: String::from("./src/sdr-lora-merged.py"),
                dev_id: 0
            }),
            consensus_config: ConsensusConfig {
                addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 5050)),
                certs: ConsensusCerts {
                    cert_path: String::from(""),
                    key_path: String::from(""),
                    ca_cert_path: String::from("")
                }
            },
        }),
        application_server: Some(ApplicationServerConfig {
            tcp_receive_port: 5050,
        }),
    };


    lazy_static! {
        static ref ARGS: Args  = Args::parse();
        static ref CONFIGS: (
            Option<Vec<DeviceConfig>>,
            Option<DeviceConfig>,
            Option<NetworkControllerConfig>,
            Option<ApplicationServerConfig>
        ) = {
            let port = 6789;
            if ARGS.config.is_none() {
                let socket =
                    TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port))
                        .unwrap();
                let mut content = String::new();
                println!("Waiting for configuration on port {port}...");
                if let Ok((mut stream, _)) = socket.accept() {
                    let mut buf = [0_u8; 1024];
                    while let Ok(v) = stream.read(&mut buf[..]) {
                        if v == 0 {
                            break;
                        } else if v < 1024 {
                            content.extend(buf[..v].iter().map(|v| char::from(*v)));
                            break;
                        } else {
                            content.extend(&buf.map(char::from));
                        }
                    }
                    println!("{}", content);
                    (serde_json::from_str::<Config>(&content))
                        .unwrap()
                        .into_configs()
                } else {
                    (None, None, None, None)
                }
            } else {
                (serde_json::from_reader::<BufReader<File>, Config>(BufReader::new(
                    File::open(Args::parse().config.unwrap()).unwrap(),
                ))
                .unwrap())
                .into_configs()
            }
        };
    }

    if let Some(c) = &CONFIGS.0 {
        device_main(c.iter().collect()).await;
    } else if let Some(c) = &CONFIGS.1 {
        device_main(vec![c]).await;
    } else if let Some(c) = &CONFIGS.2 {
        network_controller_main(c).await;
    } else if let Some(c) = &CONFIGS.3 {
        application_server_main(c).await;
    } else {
        panic!("No config provided")
    }
    Ok(())
}
