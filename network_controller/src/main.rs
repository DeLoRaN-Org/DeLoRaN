use std::net::{IpAddr, Ipv4Addr};

use blockchain_api::exec_bridge::{BlockchainExeConfig, BlockchainExeClient};
use clap::Parser;
use lazy_static::lazy_static;
use lorawan::{
    physical_parameters::{DataRate, SpreadingFactor},
    regional_parameters::region::Region,
};
use lorawan_device::{
    devices::colosseum_device::ColosseumCommunicator,
    configs::{ColosseumDeviceConfig, RadioDeviceConfig},
    devices::radio_device::RadioCommunicator,
};
use network_controller::network_controller::{NetworkController, NetworkControllerTCPConfig};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
///The Network Controller implementation for DistributedLoRaWAN
struct Args {
    /// Path of the configuration JSON file.
    #[clap(short, long, value_parser)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let _args = Args::parse();

    lazy_static! {
        static ref N_ID: String = String::from("nc_test_1");
        static ref TCP_CONFIG: NetworkControllerTCPConfig = NetworkControllerTCPConfig {
            tcp_dev_port: 9090,
            tcp_nc_port: 9091
        };
        static ref RADIO_CONFIG: RadioDeviceConfig = RadioDeviceConfig {
            region: Region::EU863_870,
            spreading_factor: SpreadingFactor::new(7),
            data_rate: DataRate::new(5),
            rx_gain: 10,
            tx_gain: 20,
            bandwidth: 125_000,
            sample_rate: 1_000_000.0,
            rx_freq: 990_000_000.0,
            tx_freq: 1_010_000_000.0,
            rx_chan_id: 0,
            tx_chan_id: 1,
            dev_id: 0
        };
        static ref COLOSSEUM_CONFIG: ColosseumDeviceConfig = ColosseumDeviceConfig {
            address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            radio_config: *RADIO_CONFIG,
            sdr_code: String::from("./src/sdr-lora-merged.py"),
        };
        static ref BC_CONFIG: BlockchainExeConfig = BlockchainExeConfig {
            orderer_addr: "orderer1.orderers.dlwan.phd:6050".to_string(),
            channel_name: "lorawan".to_string(),
            chaincode_name: "lorawan".to_string(),
            orderer_ca_file_path: None,
        };
    }

    let nc = NetworkController::new(&N_ID);
    let colosseum_routine = nc.routine::<ColosseumCommunicator, BlockchainExeClient>(
        &COLOSSEUM_CONFIG,
        &BC_CONFIG,
    );
    let tcp_routine = nc.tcp_routine::<BlockchainExeClient>(&TCP_CONFIG, &BC_CONFIG);
    let radio_routine = nc.routine::<RadioCommunicator, BlockchainExeClient>(&RADIO_CONFIG, &BC_CONFIG);

    colosseum_routine.await.unwrap();
    tcp_routine.await.unwrap();
    radio_routine.await.unwrap();

    //nc.routine().await.unwrap();
    println!("Byebye");
    Ok(())
}
