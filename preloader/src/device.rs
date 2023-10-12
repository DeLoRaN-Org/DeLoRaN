use core::panic;
use std::time::Duration;

use fake_device::{
    communicators::{ColosseumCommunication, LoRaWANCommunication},
    configs::{ColosseumDeviceConfig, DeviceConfig, DeviceConfigType},
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
impl<T: LoRaWANCommunication + Send + Sync> Run for LoRaWANDevice<T> {
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
            while let Err(e) = self.send_join_request().await {
                println!("Error joining: {e:?}");
                panic!("dio buono");
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

pub async fn device_main(configs: Vec<&'static DeviceConfig>, sdr_code: &'static str) {
    let mut handlers = Vec::new();
    let mut colosseum_communications = None;

    for config in configs {
        match &config.dtype {
            DeviceConfigType::TCP(c) => {
                handlers.push(tokio::spawn(async {
                    TcpDevice::create(config.configuration, c.addr.clone(), c.port)
                        .await
                        .run()
                        .await;
                }));
            }
            DeviceConfigType::RADIO(c) => {
                handlers.push(tokio::spawn(async {
                    RadioDevice::create(config.configuration, *c).run().await;
                }));
            }
            DeviceConfigType::COLOSSEUM(c) => {
                if colosseum_communications.is_none() {
                    colosseum_communications = Some(
                        ColosseumCommunication::new(c.address, c.radio_config, sdr_code),
                    );
                }
                let cloned = colosseum_communications.as_ref().cloned().unwrap();
                handlers.push(tokio::spawn(async {
                    LoRaWANDevice::new(
                        config.configuration,
                        cloned,
                        DeviceConfig {
                            configuration: config.configuration,
                            dtype: DeviceConfigType::COLOSSEUM(ColosseumDeviceConfig {
                                radio_config: c.radio_config,
                                address: c.address,
                            }),
                        },
                    )
                    .run()
                    .await;
                }));
                //ColosseumDevice::create(config.configuration, c.address, c.radio_config, sdr_code).run().await;
            },
            _ => {
                println!("No valid configuration if mockconfiguration is in config file")
            }
        };
        tokio::time::sleep(Duration::from_secs(17)).await;
    }

    for h in handlers {
        h.await.unwrap();
    }
}
