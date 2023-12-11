use core::panic;
use std::time::Duration;

use lorawan_device::{
    colosseum_device::{ColosseumCommunicator, ColosseumDevice},
    communicator::LoRaWANCommunicator,
    configs::{DeviceConfig, DeviceConfigType},
    debug_device::DebugDevice,
    lorawan_device::LoRaWANDevice,
    radio_device::RadioDevice,
    tcp_device::TcpDevice,
};
use tokio::time::sleep;

#[async_trait::async_trait]
pub trait Run {
    async fn run(&mut self);
}

#[async_trait::async_trait]
impl<T: LoRaWANCommunicator + Send + Sync> Run for LoRaWANDevice<T> {
    async fn run(&mut self) {
        if let Some(_s) = self.session() {
            println!("Device already initialized:");
            //println!("{s}");
        } else {
            println!(
                "Device {} needs initialization, sending join request...",
                self.dev_eui()
            );

            self.set_dev_nonce(0);

            for i in 0..3 {
                match self.send_join_request().await {
                    Ok(_) => {
                        println!("{} joined after {i} tries", self.dev_eui());
                        //println!("{}", **self);
                        break;
                    }
                    Err(e) => {
                        println!("Error while sending join request: {:?}", e);
                        if i == 2 {
                            panic!("Error cannot join: {:?}", e);
                        }
                    }
                }
            }
        }

        let duration = 91_u64;
        let mut errors = 0;
        for i in 0..30 {
            sleep(Duration::from_secs(duration)).await;
            match self
                .send_uplink(
                    Some(format!("confirmed {i} di {i} prova").as_bytes()),
                    true,
                    Some(1),
                    None,
                )
                .await
            {
                Ok(_) => {
                    //println!("Uplink sent");
                }
                Err(e) => {
                    println!("Error while sending uplink: {:?}", e);
                    errors += 1;
                }
            }
        }
        println!("Device {} ending, {} errors", self.dev_eui(), errors);
    }
}

pub async fn device_main(configs: Vec<&'static DeviceConfig>) {
    let mut handlers = Vec::new();
    let mut colosseum_communications = None;
    const SKIP_DEVICES: usize = 0;
    const LIMIT: usize = 500;

    if SKIP_DEVICES + LIMIT > configs.len() {
        panic!("Not enough devices in config file");
    }
    
    let num_devices: usize = if configs.len() < LIMIT { configs.len() } else { LIMIT };

    for (i, config) in configs[SKIP_DEVICES..(SKIP_DEVICES + num_devices)].iter().enumerate() {
        match &config.dtype {
            DeviceConfigType::TCP(c) => {
                handlers.push(tokio::spawn(async {
                    DebugDevice::from(TcpDevice::create(config.configuration, c).await)
                        .run()
                        .await;
                }));
            }
            DeviceConfigType::RADIO(c) => {
                handlers.push(tokio::spawn(async {
                    DebugDevice::from(RadioDevice::create(config.configuration, c).await)
                        .run()
                        .await;
                }));
            }
            DeviceConfigType::COLOSSEUM(c) => {
                if colosseum_communications.is_none() {
                    colosseum_communications =
                        Some(*ColosseumCommunicator::from_config(c).await.unwrap());
                }
                let cloned = colosseum_communications.as_ref().cloned().unwrap();

                handlers.push(tokio::spawn(async {
                    DebugDevice::from(
                        ColosseumDevice::with_shared_communicator(config.configuration, cloned)
                            .await,
                    )
                    .run()
                    .await;
                }));
                //ColosseumDevice::create(config.configuration, c.address, c.radio_config, sdr_code).run().await;
            }
            _ => {
                println!("Not a valid configuration if mockconfiguration is in config file")
            }
        };
        println!("Device {i} created");
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    for (i, h) in handlers.into_iter().enumerate() {
        if let Err(e) = h.await {
            println!("Device [{i}] ended with error: {:?}", e);
        };
    }
}
