use std::{fs, net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4}};

use blockchain_api::{exec_bridge::{BlockchainExeClient, BlockchainExeConfig}, udp_bridge::Logger};
use clap::Parser;
use consensus::{consensus_server::ConsensusConfig, ConsensusCerts};
use lazy_static::lazy_static;
use lorawan::{
    physical_parameters::{CodeRate, DataRate, LoRaBandwidth, SpreadingFactor},
    regional_parameters::region::Region,
};
use lorawan_device::{configs::{ColosseumDeviceConfig, RadioDeviceConfig, UDPNCConfig}, devices::radio_device::RadioCommunicator};

use nalgebra::{DMatrix, DVector};
use network_controller::modules::{anomaly_detector_mahalanobis::AnomalyDetectorMahalnobis, network_controller::NetworkController};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
///The Network Controller implementation for DistributedLoRaWAN
struct Args {
    /// Path of the configuration JSON file.
    #[clap(short, long, value_parser)]
    config: Option<String>,
}

lazy_static! {
    static ref LOGGER: Logger = Logger::new("anomaly_detector_mahalanobis_test.csv", true, false);
}

/*
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

*/
#[test]
fn test_update_mean() {
    let mut detector = AnomalyDetectorMahalnobis::new(5.0, 3);
    let sf = SpreadingFactor::new(7);
    let frequency = 868_100_000;

    let row = DVector::from_vec(vec![1.0, 4.0, 3.0]);
    detector.update(sf, frequency.to_string(), &row);
    let row = DVector::from_vec(vec![1.0, 2.0, 3.0]);
    detector.update(sf, frequency.to_string(), &row);
    let row = DVector::from_vec(vec![4.0, 6.0, 3.0]);
    detector.update(sf, frequency.to_string(), &row);
    assert_eq!(detector.mean(sf, frequency.to_string()), &DVector::from_vec(vec![2.0, 4.0, 3.0]));
}

fn test_real_data(threshold: f64) -> (f32, f32, f32, f32) {
    let mut ad = AnomalyDetectorMahalnobis::new(threshold, 3);
    
    let path = "/home/rastafan/Documenti/Dottorato/code/DeLoRaN/DeLoRaN/network_controller/transformed_gateway_with_jammer_status.csv";
    let content = fs::read_to_string(path).unwrap();
    let rows = content.lines().collect::<Vec<_>>();

    let mut row_count = 0;
    let mut false_positives = 0;
    let mut false_negatives = 0;
    let mut true_positives = 0;
    let mut true_negatives = 0;

    for row in rows.iter().skip(1) {
        let values = row.split(',').collect::<Vec<&str>>();
        //println!("{values:?}");

        //Time,DeviceAddress,GatewayNode,FrequencyHz,SF,BW,RSSI,SNR,Distance,Jammer
        //1.22171,6c00079d,202,868300000,9,125000,-128.173,-11.1419,3770.55,0

        let time = values[0].parse::<f32>().unwrap() as u128;
        let frequency = values[3].parse::<f64>().unwrap();
        let sf = SpreadingFactor::new(values[4].parse::<u8>().unwrap());
        //let bw = LoRaBandwidth::from(values[5].parse::<f32>().unwrap());
        let rssi = values[6].parse::<f32>().unwrap();
        let snr = values[7].parse::<f32>().unwrap();
        let jammer = values[9].parse::<u8>().unwrap() == 1;
        
        row_count += 1;
        if row_count < 4000 {
            ad.update(sf, frequency.to_string(), &DVector::from_vec(vec![rssi as f64, snr as f64, time as f64]));

            println!("{}, {:?}, {:?}, {:?}", ad.get_mahalanobi(sf, frequency.to_string()).get_counter(), ad.mean(sf, frequency.to_string()).data.as_slice(), ad.variance(sf, frequency.to_string()).data.as_slice(), ad.covariance_matrix(sf, frequency.to_string()).data.as_slice());
            continue;
        }

        //ad.update(sf, frequency as u64, &DVector::from_vec(vec![rssi as f64, snr as f64, time as f64]));
        let (is_anomaly, distance) = ad.is_anomaly(sf, frequency.to_string(),&DVector::from_vec(vec![rssi as f64, snr as f64, time as f64]));
        if !is_anomaly {
            ad.update(sf, frequency.to_string(), &DVector::from_vec(vec![rssi as f64, snr as f64, time as f64]));
        }
        
        //LOGGER.write_sync(&format!("{}, {}, {}, {}", distance, row_count, jammer, is_anomaly));
        row_count += 1;
        
        match (jammer, is_anomaly) {
            (true, true) => true_positives += 1,
            (true, false) => false_negatives += 1,
            (false, true) => false_positives += 1,
            (false, false) => true_negatives += 1,
        }
    }

    let path = "/home/rastafan/Documenti/Dottorato/code/DeLoRaN/DeLoRaN/gatewayJammed2.csv";
    let content = fs::read_to_string(path).unwrap();
    let rows = content.lines().collect::<Vec<_>>();

    for row in rows.iter().skip(1) {
        let values = row.split(',').collect::<Vec<&str>>();
        //println!("{values:?}");

        let time = values[0].parse::<f32>().unwrap() as u128;
        let frequency = values[3].parse::<f64>().unwrap();
        let sf = SpreadingFactor::new(values[4].parse::<u8>().unwrap());
        //let bw = LoRaBandwidth::from(values[5].parse::<f32>().unwrap());
        let rssi = values[6].parse::<f32>().unwrap();
        let snr = values[7].parse::<f32>().unwrap();
        let jammer = values[9].parse::<u8>().unwrap() == 1;

        //println!("{}", row_count);
        //if row_count == 94201 {
        //    println!("{:?}", ad);
        //    println!("{:?}", ad.mean(sf, frequency.to_string()));
        //    println!("{:?}", ad.covariance_matrix(sf, frequency.to_string()));
        //    println!("{:?}", ad.get_mahalanobi(sf, frequency.to_string()));
        //}
        let (is_anomaly, distance) = ad.is_anomaly(sf, frequency.to_string(),&DVector::from_vec(vec![rssi as f64, snr as f64, time as f64]));
        if !is_anomaly {
            ad.update(sf, frequency.to_string(), &DVector::from_vec(vec![rssi as f64, snr as f64, time as f64]));
        }

        LOGGER.write_sync(&format!("{}, {}, {}, {}", distance, row_count, jammer, is_anomaly));
        row_count += 1;
        
        match (jammer, is_anomaly) {
            (true, true) => true_positives += 1,
            (true, false) => false_negatives += 1,
            (false, true) => false_positives += 1,
            (false, false) => true_negatives += 1,
        }
    }

    
    let round_precision = true_positives as f32 / (true_positives + false_positives) as f32;
    let round_recall = true_positives as f32 / (true_positives + false_negatives) as f32;
    let f1_score = 2.0 * (round_precision * round_recall) / (round_precision + round_recall);
    let round_accuracy = (true_positives + true_negatives) as f32 / (true_positives + true_negatives + false_positives + false_negatives) as f32;
    
    


    //println!("Jammer Count: {}, Not Jammer Count: {}", jammer_count, not_jammer_count);

    
    //println!("True Positives: {}, True Negatives: {}, False Positives: {}, False Negatives: {}", true_positives, true_negatives, false_positives, false_negatives);
    //println!("Precision: {}, Recall: {}, Accuracy: {}, f1: {}", round_precision, round_recall, round_accuracy, f1_score);
    (
        round_precision,
        round_recall,
        f1_score,
        round_accuracy,
    )
}


fn main() {
    let mut threshold = 1.0;
    let stop = 1.0;
    let step = 0.5;

    let mut best_threshold = 0.0;
    let mut best_precision = 0.0;
    let mut best_recall = 0.0;
    let mut best_f1 = 0.0;
    let mut best_accuracy = 0.0;


    while threshold <= stop {
        let (precision, recall, f1_score, accuracy) = test_real_data(threshold);

        if precision > best_precision {
            best_threshold = threshold;
            best_precision = precision;
            best_recall = recall;
            best_f1 = f1_score;
            best_accuracy = accuracy;
        }

        threshold += step;
    }

    //println!("Best Threshold: {}, Precision: {}, Recall: {}, F1: {}, Accuracy: {}", best_threshold, best_precision, best_recall, best_f1, best_accuracy);
}