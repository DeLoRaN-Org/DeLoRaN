use core::panic;
use std::time::Duration;

use lorawan_device::{
    communicator::LoRaWANCommunicator,
    configs::{DeviceConfig, DeviceConfigType},
    lorawan_device::LoRaWANDevice,
    radio_device::RadioDevice,
    tcp_device::TcpDevice, colosseum_device::{ColosseumDevice, ColosseumCommunicator}, debug_device::DebugDevice,
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
            //let duration = 10;
            //sleep(Duration::from_secs(duration)).await;
            self.set_dev_nonce(7);
            while let Err(e) = self.send_join_request().await {
                panic!("Error joining: {e:?}");
            };
            println!("{}", **self);
        }

        let duration = 90_u64;
        for _ in 0..1 {
            for i in 0..100 {
                self.send_uplink(
                    Some(format!("confirmed {i} di {i} prova").as_bytes()),
                    true,
                    Some(1),
                    None,
                )
                .await
                .unwrap();
                sleep(Duration::from_secs(duration)).await;
            }
        }
    }
}

pub async fn device_main(configs: Vec<&'static DeviceConfig>) {
    let mut handlers = Vec::new();
    let mut colosseum_communications = None;

    for config in configs {
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
                    DebugDevice::from(RadioDevice::create(config.configuration, c).await).run().await;
                }));
            }
            DeviceConfigType::COLOSSEUM(c) => {
                if colosseum_communications.is_none() {
                    colosseum_communications = Some(
                        *ColosseumCommunicator::from_config(c).await.unwrap(),
                    );
                }
                let cloned = colosseum_communications.as_ref().cloned().unwrap();


                handlers.push(tokio::spawn(async {
                    DebugDevice::from(ColosseumDevice::with_shared_communicator(config.configuration, cloned).await)
                    .run().await;
                }));
                //ColosseumDevice::create(config.configuration, c.address, c.radio_config, sdr_code).run().await;
            },
            _ => {
                println!("Not a valid configuration if mockconfiguration is in config file")
            }
        };
        tokio::time::sleep(Duration::from_secs(17)).await;
    }

    for h in handlers {
        h.await.unwrap();
    }
}
