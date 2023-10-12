use std::time::Duration;

use clap::Parser;
use fake_device::tcp_device::TcpDevice;
use lorawan::utils::eui::EUI64;
use tokio::time::sleep;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
///The Network Controller implementation for DistributedLoRaWAN
struct Args {
    /// Path of the configuration JSON file.
    #[clap(short, long, value_parser)]
    config: Option<String>,
}

#[tokio::main]
async fn main() {
    //let _args = Args::parse();
    
    let dev_eui = EUI64::from_hex("50DE2646F9A7AC8E").unwrap();
    let mut device = TcpDevice::from_blockchain(&dev_eui, "localhost".to_owned(), 9090).await;

    if let Some(s) = device.session() {
        println!("Device already initialized:");
        println!("{s}");
    } else {
        println!("Device needs initialization, sending join request...");
        device.send_join_request().await;
    }

    for _ in 0..1 {
        for i in 0..1 {
            device.send_uplink(Some(format!("unconfirmed {i} message").as_bytes()), false, Some(1), None).await.unwrap();
        }

        sleep(Duration::from_secs(2)).await;
        
        for i in 0..1 {
            device.send_uplink(Some(format!("confirmed {i} di {i} prova").as_bytes()), true, Some(1), None).await.unwrap();
        }     
        
        //for _ in 0..1 {
        //    device.send_maccommands(&[EDMacCommands::ResetInd(1),
        //        EDMacCommands::DeviceTimeReq,
        //        //EDMacCommands::RXParamSetupAns {
        //        //    rx1_dr_offset_ack: true,
        //        //    rx2_data_rate_ack: false,
        //        //    channel_ack: false,
        //        //},
        //        //EDMacCommands::DevStatusAns {
        //        //    battery: 200,
        //        //    margin: 10,
        //        //},
        //        //EDMacCommands::DlChannelAns {
        //        //    uplink_frequency_exists: true,
        //        //    channel_frequency_ok: true,
        //        //}
        //    ], true).await.unwrap();
        //}

        //let packet = LoRaWANPacket::from_bytes(&Vec::from_hex("20E775ED5536E87C3EE9FA5B86AF1834BE27417EC6CA54ECEFBDD5AD61B3713664").unwrap(), Some(&device), false).unwrap();
        //println!("{packet:?}");
    }
}
