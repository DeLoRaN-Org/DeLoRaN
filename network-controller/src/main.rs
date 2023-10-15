use std::net::{IpAddr, Ipv4Addr};

use clap::Parser;
use lorawan_device::configs::{RadioDeviceConfig, ColosseumDeviceConfig};
use lazy_static::lazy_static;
use lorawan::{regional_parameters::region::Region, physical_parameters::{SpreadingFactor, DataRate}};
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
        };
    }

    let nc = NetworkController::init(&N_ID, Some(&TCP_CONFIG), Some(&RADIO_CONFIG), Some(&COLOSSEUM_CONFIG));
    nc.routine(None).await.unwrap();
    println!("Byebye");
    Ok(())
}
