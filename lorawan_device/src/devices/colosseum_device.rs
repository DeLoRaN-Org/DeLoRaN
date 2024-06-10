use std::ops::{Deref, DerefMut};
use std::fmt::Debug;
use std::{fs, thread};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use blockchain_api::BlockchainClient;
use blockchain_api::exec_bridge::BlockchainExeClient;
use lorawan::physical_parameters::{CodeRate, SpreadingFactor};
use lorawan::{device::Device, utils::eui::EUI64};
use pyo3::{PyAny, Python, Py, prelude::PyAnyMethods};
use pyo3::types::PyModule;
use tokio::sync::{oneshot, mpsc};

use pyo3::prelude::*;


use crate::communicator::{extract_dev_id, ArrivalStats, CommunicatorError, LoRaWANCommunicator, Position, ReceivedTransmission, Transmission};
use crate::configs::{RadioDeviceConfig, ColosseumDeviceConfig};
use crate::devices::lorawan_device::LoRaWANDevice;


#[derive(Debug, Default, Clone)]
pub struct PyLoRaPacket {
    pub payload: Vec<u8>,
    pub src: u16,
    pub dst: u16,
    pub seqn: u8,
    pub hdr_ok: u8,
    pub has_crc: u8,
    pub crc_ok: u8,
    pub cr: u8,
    pub ih: u8,
    pub sf: u8,
    pub bw: f32,
    pub rssi: f32,
    pub snr: f32,
}

impl<'source> FromPyObject<'source> for PyLoRaPacket {
    fn extract(ob: &'source PyAny) -> PyResult<Self> {
        Ok(Self {
            payload: ob.getattr("payload")?.extract()?,
            src: ob.getattr("src")?.extract()?,
            dst: ob.getattr("dst")?.extract()?,
            seqn: ob.getattr("seqn")?.extract()?,
            hdr_ok: ob.getattr("hdr_ok")?.extract()?,
            has_crc: ob.getattr("has_crc")?.extract()?,
            crc_ok: ob.getattr("crc_ok")?.extract()?,
            cr: ob.getattr("cr")?.extract()?,
            ih: ob.getattr("ih")?.extract()?,
            sf: ob.getattr("SF")?.extract()?,
            bw: ob.getattr("BW")?.extract()?,
            rssi: ob.getattr("rssi")?.extract()?,
            snr: ob.getattr("snr")?.extract()?,
        })
    }
}

impl From<PyLoRaPacket> for ReceivedTransmission {
    fn from(packet: PyLoRaPacket) -> Self {
        Self {
            transmission: Transmission {
                start_position: Position { x: 0.0, y: 0.0, z: 0.0 },
                start_time: 0,
                frequency: 868_000_000.0,
                bandwidth: packet.bw.into(),
                spreading_factor: SpreadingFactor::new(packet.sf),
                code_rate: CodeRate::CR4_5,
                starting_power: packet.rssi,
                uplink: false,
                payload: packet.payload,
            },
            arrival_stats: ArrivalStats {
                time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
                rssi: packet.rssi,
                snr: packet.snr,
            },
        }
    }
}

pub struct ColosseumDevice {
    device: LoRaWANDevice<ColosseumCommunicator>,
}

impl ColosseumDevice {
    pub async fn create(device: Device, config: &ColosseumDeviceConfig) -> LoRaWANDevice<ColosseumCommunicator> {
        let mut c = ColosseumCommunicator::from_config(config).await.unwrap();
        if let Err(e) =  c.register_device(*device.dev_eui()).await {
            eprintln!("{e:?}")
        } 
        LoRaWANDevice::new(device, c)
    }
    
    pub async fn with_shared_communicator(device: Device,  mut communicator: ColosseumCommunicator) -> LoRaWANDevice<ColosseumCommunicator> {
        if let Err(e) = communicator.register_device(*device.dev_eui()).await {
            eprintln!("{e:?}")
        }
        LoRaWANDevice::new(device, communicator)
    }

    pub async fn from_blockchain(dev_eui: &EUI64, config: &ColosseumDeviceConfig) -> LoRaWANDevice<ColosseumCommunicator> {
        let client = BlockchainExeClient::new("orderer1.orderers.dlwan.phd:6050", "lorawan", "lorawan", None);
        let device = client.get_device(dev_eui).await.unwrap();

        let mut c = ColosseumCommunicator::from_config(config).await.unwrap();
        if let Err(e) =  c.register_device(*device.dev_eui()).await {
            eprintln!("{e:?}")
        }
        LoRaWANDevice::new(device, c)
    }
}

impl Deref for ColosseumDevice {
    type Target=LoRaWANDevice<ColosseumCommunicator>;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl DerefMut for ColosseumDevice {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.device
    }
}

impl Debug for ColosseumDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ColosseumDevice").field("device", &self.device).finish()
    }
}


type DownlinkRequest = (Option<u16>, Option<Duration>);

enum ReceiverReq {
    ReceiveDownlink(DownlinkRequest),
    RegisterDevice(u16)
}

type Downlinks = (bool, Vec<(u8, PyLoRaPacket)>);
enum ReceiverAns {
    ReceiveDownlink(Downlinks),
    RegisterDevice(bool)
}


type SenderArgs = (Vec<u8>, Option<EUI64>, Option<EUI64>);
type SenderAns = bool;

type SenderChannel = (SenderArgs, oneshot::Sender<SenderAns>);
type ReceiverChannel = (ReceiverReq, oneshot::Sender<ReceiverAns>);

#[derive(Clone, Debug)]
pub struct ColosseumCommunicator {
    sender_send: mpsc::Sender<SenderChannel>,
    receiver_send: mpsc::Sender<ReceiverChannel>,
    radio_config: RadioDeviceConfig,
    dev_id: u16,
}

impl ColosseumCommunicator {
    pub async fn register_device(&mut self, d_id: EUI64) ->  Result<(), CommunicatorError> {
        let (send, recv) = oneshot::channel();
        let dev_id = extract_dev_id(Some(d_id));
        //println!("Registering device: {dev_id}");
        let _ = self.receiver_send.send((ReceiverReq::RegisterDevice(dev_id), send)).await;
        match recv.await {
            Ok(r) => {
                if let ReceiverAns::RegisterDevice(r) = r {
                    if r {
                        println!("Succesfully registered device: {d_id}/{dev_id}");
                        self.dev_id = dev_id;
                        Ok(())
                    } else {
                        Err(CommunicatorError::Radio(
                            "Unable to send message".to_string(),
                        ))
                    }
                } else {
                    unreachable!("should never reach this point in register device");
                }
            }
            Err(e) => {
                eprintln!("{e:?}");
                Err(CommunicatorError::Radio(
                    "Cannot send command to radio thread".to_string(),
                ))
            }
        }
    }

    pub async fn change_config(&mut self, config: &RadioDeviceConfig) -> Result<(), CommunicatorError> {
        self.radio_config = *config;
        todo!("Change config in the radio thread"); //TODO find a way to reflect changes of the radio config in the real radio in the radio thread
    }
}

impl LoRaWANCommunicator for ColosseumCommunicator {
    type Config = ColosseumDeviceConfig;
    
    async fn from_config(config: &ColosseumDeviceConfig) -> Result<Self, CommunicatorError> {
        let radio_config = config.radio_config;
        let (sender_send, mut sender_recv) =
            mpsc::channel::<SenderChannel>(200);
        let (receiver_send, mut receiver_recv) =
            mpsc::channel::<ReceiverChannel>(200);
        let sdr_lora_code = fs::read_to_string(&config.sdr_code).unwrap();
        let (lora_sender, lora_receiver): (Py<PyAny>, Py<PyAny>) = Python::with_gil(|py| {
            let sdr_module =
                PyModule::from_code_bound(py, &sdr_lora_code, "sdr-lora-merged.py", "sdr-lora").unwrap();
            sdr_module
                .getattr("LoRaBufferedBuilder")
                .unwrap()
                .call(
                    (
                        config.address.to_string(),
                        radio_config.bandwidth.hz(),
                        radio_config.freq,
                        radio_config.freq,
                        radio_config.sample_rate,
                        radio_config.rx_chan_id,
                        radio_config.tx_chan_id,
                        radio_config.spreading_factor.value(),
                    ),
                    None,
                )
                .unwrap()
                .extract()
                .unwrap()
        });
        
        thread::spawn(move || {
            while let Some(((data, src, dest), sender)) = sender_recv.blocking_recv() {
                //println!("{}", PrettyHexSlice(&data));
                Python::with_gil(|py| {
                    match lora_sender.call_method_bound(
                        py,
                        "send_radio",
                        (data, extract_dev_id(src), extract_dev_id(dest)),
                        None,
                    ) {
                        Ok(_) => {
                            let _ = sender.send(true);
                        }
                        Err(e) => {
                            eprintln!("{e}");
                            let _ = sender.send(false);
                        }
                    }
                });
            }
            println!("Thread sender died");
        });
        
        thread::spawn(move || {
            while let Some((req, sender)) = receiver_recv.blocking_recv() {
                match req {
                    ReceiverReq::ReceiveDownlink((d_id, timeout)) => {
                        let sf_list = [radio_config.spreading_factor.value()];
                        Python::with_gil(|py| {
                            match lora_receiver
                            .call_method_bound(py, "recv_radio", (sf_list, d_id ,timeout.map(|d| d.as_secs())), None)
                            .unwrap()
                            .extract(py)
                            {
                                Ok(v) => {
                                    let _ = sender.send(ReceiverAns::ReceiveDownlink((true, v)));
                                }
                                Err(e) => {
                                    eprintln!("{e}");
                                }
                            };
                        });
                    },
                    ReceiverReq::RegisterDevice(d_id) => {
                        Python::with_gil(|py| {
                            lora_receiver
                            .call_method_bound(py, "register_device_id", (d_id,), None)
                            .unwrap();
                            let _ = sender.send(ReceiverAns::RegisterDevice(true));
                        });
                    },
                }
            }
            println!("Thread receiver died");
        });
        
        Ok(Self {
            sender_send,
            receiver_send,
            radio_config,
            dev_id: config.dev_id,
        })
    }

    async fn send(
        &self,
        bytes: &[u8],
        src: Option<EUI64>,
        dest: Option<EUI64>,
    ) -> Result<(), CommunicatorError> {
        let (send, recv) = oneshot::channel();
        let _ = self.sender_send.send(((bytes.to_vec(), src, dest), send)).await;
        match recv.await {
            Ok(r) => {
                if r {
                    Ok(())
                } else {
                    Err(CommunicatorError::Radio(
                        "Unable to send message".to_string(),
                    ))
                }
            }
            Err(e) => {
                eprintln!("{e:?}");
                Err(CommunicatorError::Radio(
                    "Cannot send command to radio thread".to_string(),
                ))
            }
        }
    }

    async fn receive(
        &self,
        timeout: Option<Duration>,
    ) -> Result<Vec<ReceivedTransmission>, CommunicatorError> {
        let (send, recv) = oneshot::channel();
        let _ = self
            .receiver_send
            .send((ReceiverReq::ReceiveDownlink((Some(self.dev_id), timeout)), send))
            .await;

        match recv.await {
            Ok(res) => {
                if let ReceiverAns::ReceiveDownlink((r, buffers)) = res {
                    if r {
                        Ok(buffers
                            .into_iter()
                            .map(|(_, p)| (ReceivedTransmission::from(p)))
                            .collect())
                    } else {
                        Err(CommunicatorError::Radio(
                            "Error receiving packets".to_string(),
                        ))
                    }
                } else {
                    unreachable!("should never reach this point in receive_downlink");
                }
            }
            Err(e) => {
                eprintln!("{e:?}");
                Err(CommunicatorError::Radio(
                    "Error sending command to radio thread".to_string(),
                ))
            }
        }
    }
}