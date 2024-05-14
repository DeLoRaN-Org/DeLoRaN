use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};

use blockchain_api::exec_bridge::{BlockchainExeConfig, BlockchainExeClient};
use clap::Parser;
use consensus::{consensus_server::ConsensusConfig, ConsensusCerts};
use lazy_static::lazy_static;
use lorawan::{
    physical_parameters::{CodeRate, DataRate, LoRaBandwidth, SpreadingFactor},
    regional_parameters::region::Region,
};
use lorawan_device::{configs::{ColosseumDeviceConfig, RadioDeviceConfig, UDPNCConfig}, devices::radio_device::RadioCommunicator};

use network_controller::modules::network_controller::NetworkController;

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
        static ref NC_ID: String = String::from("nc_test_1");
        static ref UDP_CONFIG: UDPNCConfig = UDPNCConfig { 
            addr: "0.0.0.0".to_owned(),
            port: 9090
        };
        static ref RADIO_CONFIG: RadioDeviceConfig = RadioDeviceConfig {
            region: Region::EU863_870,
            spreading_factor: SpreadingFactor::SF7,
            data_rate: DataRate::DR5,
            bandwidth: LoRaBandwidth::BW125,
            sample_rate: 1_000_000.0,
            freq: 990_000_000.0,
            rx_chan_id: 0,
            tx_chan_id: 1,
            code_rate: CodeRate::CR4_5
        };
        static ref COLOSSEUM_CONFIG: ColosseumDeviceConfig = ColosseumDeviceConfig {
            address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            radio_config: *RADIO_CONFIG,
            sdr_code: String::from("./src/sdr-lora-merged.py"),
            dev_id: 0
        };
        static ref BC_CONFIG: BlockchainExeConfig = BlockchainExeConfig {
            orderer_addr: "orderer1.orderers.dlwan.phd:6050".to_string(),
            channel_name: "lorawan".to_string(),
            chaincode_name: "lorawan".to_string(),
            orderer_ca_file_path: None,
        };
        
        static ref CONSENSUS_CONFIG: ConsensusConfig = ConsensusConfig {
            addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 5050)),
            certs: ConsensusCerts {
                cert_path: String::from(""),
                key_path: String::from(""),
                ca_cert_path: String::from("")
            }
        };
    }

    let nc = NetworkController::new(&NC_ID, CONSENSUS_CONFIG.clone());
    //let colosseum_routine = nc.routine::<ColosseumCommunicator, BlockchainExeClient>(
    //    &COLOSSEUM_CONFIG,
    //    &BC_CONFIG,
    //);
    let tcp_routine = nc.udp_routine::<BlockchainExeClient>(&UDP_CONFIG, &BC_CONFIG);
    let radio_routine = nc.routine::<RadioCommunicator, BlockchainExeClient>(&RADIO_CONFIG, &BC_CONFIG);
    
    //colosseum_routine.await.unwrap();
    tcp_routine.await.unwrap();
    radio_routine.await.unwrap();

    println!("Byebye");
    Ok(())
}

/*
#[cfg(test)]
mod tests {
    use std::time::Duration;

    use lorawan_device::{communicator::LoRaWANCommunicator, configs::MockDeviceConfig, devices::{debug_device::DebugCommunicator, mock_device::MockCommunicator}};
    use network_controller::modules::downlink_scheduler::{self, DownlinkSchedulerMessage};
    use tokio::time::Instant;

    #[tokio::test]
    async fn test() {
        let config = MockDeviceConfig {};
        let communicator = DebugCommunicator::from(MockCommunicator::from_config(&config).await.unwrap(), None);
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        
        let mut downlink_scheduler = downlink_scheduler::DownlinkScheduler::new(communicator, rx).await;
        tokio::spawn(async move { downlink_scheduler.run().await });

        tx.send(DownlinkSchedulerMessage { payload: vec![1,1,1,1,1,1,1,1], moment: Instant::now() + Duration::from_secs(5) }).await.unwrap();
        tx.send(DownlinkSchedulerMessage { payload: vec![0,0,0,0,0,0,0,0], moment: Instant::now() + Duration::from_secs(1) }).await.unwrap();
        tx.send(DownlinkSchedulerMessage { payload: vec![3,3,3,3,3,3,3,3], moment: Instant::now() + Duration::from_secs(20) }).await.unwrap();
        tx.send(DownlinkSchedulerMessage { payload: vec![2,2,2,2,2,2,2,2], moment: Instant::now() + Duration::from_secs(10) }).await.unwrap();
        
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}
 */