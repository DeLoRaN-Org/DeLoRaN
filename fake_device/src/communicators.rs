use std::{collections::HashMap, net::IpAddr, thread, time::Duration};

use pyo3::prelude::*;

use crate::configs::RadioDeviceConfig;
use async_trait::async_trait;
use lorawan::{
    physical_parameters::SpreadingFactor,
    utils::{errors::LoRaWANError, eui::EUI64, PrettyHexSlice},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{mpsc, oneshot},
};

pub fn extract_dev_id(dev_eui: Option<EUI64>) -> u8 {
    match dev_eui {
        Some(v) => {
            let prime: u64 = 31;
            let mut hash: u64 = 0;
            for &value in (*v).iter() {
                hash = hash.wrapping_mul(prime);
                hash = hash.wrapping_add(u64::from(value));
            }
            let hash_bytes = hash.to_ne_bytes();
            let mut folded_hash: u8 = 0;
            for &value in hash_bytes.iter() {
                folded_hash ^= value;
            }
            if folded_hash == 0 {
                folded_hash.wrapping_add(1)
            } else {
                folded_hash
            }
        }
        None => 0,
    }
}

#[derive(Debug)]
pub enum CommunicationError {
    Radio(String),
    TCP(std::io::Error),
    LoRaWANError(LoRaWANError),
}

#[async_trait]
pub trait LoRaWANCommunication {
    async fn send_uplink(
        &mut self,
        bytes: &[u8],
        src: Option<EUI64>,
        dest: Option<EUI64>,
    ) -> Result<(), CommunicationError>;
    async fn receive_downlink(
        &mut self,
        d_id: Option<EUI64>, //FIXME zozzata per far combaciare pythone e rust al volo ma bisogna fare un oggetto proxy per colosseumcommunication che condivide le queue ma non le configurazioni e setta send e recv giuste.
        timeout: Option<Duration>,
    ) -> Result<HashMap<SpreadingFactor, LoRaPacket>, CommunicationError>;
}

impl From<LoRaWANError> for CommunicationError {
    fn from(value: LoRaWANError) -> Self {
        CommunicationError::LoRaWANError(value)
    }
}

impl From<std::io::Error> for CommunicationError {
    fn from(value: std::io::Error) -> Self {
        CommunicationError::TCP(value)
    }
}

pub struct TCPCommunication {
    stream: TcpStream,
}

impl From<TcpStream> for TCPCommunication {
    fn from(stream: TcpStream) -> Self {
        Self { stream }
    }
}

#[async_trait]
impl LoRaWANCommunication for TCPCommunication {
    async fn send_uplink(
        &mut self,
        bytes: &[u8],
        _src: Option<EUI64>,
        _dest: Option<EUI64>,
    ) -> Result<(), CommunicationError> {
        Ok(self.stream.write_all(bytes).await?)
    }

    async fn receive_downlink(
        &mut self,
        _d_id: Option<EUI64>,
        _timeout: Option<Duration>,
    ) -> Result<HashMap<SpreadingFactor, LoRaPacket>, CommunicationError> {
        let mut buf = Vec::with_capacity(100);
        let _len = self.stream.read_buf(&mut buf).await?;

        let packet = LoRaPacket {
            payload: buf,
            ..Default::default()
        };
        Ok(HashMap::from([(SpreadingFactor::new(7), packet)]))
    }
}

#[derive(Clone, Copy)]
pub struct RadioCommunication {
    pub config: RadioDeviceConfig,
}

impl RadioCommunication {
    pub fn new(config: RadioDeviceConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl LoRaWANCommunication for RadioCommunication {
    async fn send_uplink(
        &mut self,
        _bytes: &[u8],
        _src: Option<EUI64>,
        _dest: Option<EUI64>,
    ) -> Result<(), CommunicationError> {
        todo!()
    }
    
    async fn receive_downlink(
        &mut self,
        _d_id: Option<EUI64>,
        _timeout: Option<Duration>,
    ) -> Result<HashMap<SpreadingFactor, LoRaPacket>, CommunicationError> {
        todo!()
    }
}

#[derive(Debug, Default, Clone)]
pub struct LoRaPacket {
    pub payload: Vec<u8>,
    pub src: u8,
    pub dst: u8,
    pub seqn: u8,
    pub hdr_ok: u8,
    pub has_crc: u8,
    pub crc_ok: u8,
    pub cr: u8,
    pub ih: u8,
    pub sf: u8,
    pub bw: f32,
}

impl<'source> FromPyObject<'source> for LoRaPacket {
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
        })
    }
}

type ReceiverArgs = (Option<EUI64>, Option<Duration>);
type ReceiverAns = (bool, Vec<(u8, LoRaPacket)>);

type SenderArgs = (Vec<u8>, Option<EUI64>, Option<EUI64>);
type SenderAns = bool;

type SenderChannel = (SenderArgs, oneshot::Sender<SenderAns>);
type ReceiverChannel = (ReceiverArgs, oneshot::Sender<ReceiverAns>);

#[derive(Clone, Debug)]
pub struct ColosseumCommunication {
    sender_send: mpsc::Sender<SenderChannel>,
    receiver_send: mpsc::Sender<ReceiverChannel>,
}

impl ColosseumCommunication {
    pub fn new(
        colosseum_address: IpAddr,
        radio_config: RadioDeviceConfig,
        sdr_lora_code: &'static str,
    ) -> Self {
        let (sender_send, mut sender_recv) =
            mpsc::channel::<SenderChannel>(200);
        let (receiver_send, mut receiver_recv) =
            mpsc::channel::<ReceiverChannel>(200);
        
        let (lora_sender, lora_receiver): (Py<PyAny>, Py<PyAny>) = Python::with_gil(|py| {
            let sdr_module =
                PyModule::from_code(py, sdr_lora_code, "sdr-lora-merged.py", "sdr-lora").unwrap();
            sdr_module
                .getattr("LoRaBufferedBuilder")
                .unwrap()
                .call(
                    (
                        colosseum_address.to_string(),
                        radio_config.rx_gain,
                        radio_config.tx_gain,
                        radio_config.bandwidth,
                        radio_config.rx_freq,
                        radio_config.tx_freq,
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
                println!("{}", PrettyHexSlice(&data));
                Python::with_gil(|py| {
                    match lora_sender.call_method(
                        py,
                        "send_radio",
                        (data, extract_dev_id(src), extract_dev_id(dest)),
                        None,
                    ) {
                        Ok(_) => {
                            println!("Sent!");
                            let _ = sender.send(true);
                        }
                        Err(e) => {
                            println!("{e}");
                            let _ = sender.send(false);
                        }
                    }
                });
            }
            println!("Thread sender died");
        });
        
        thread::spawn(move || {
            while let Some(((d_id, timeout), sender)) = receiver_recv.blocking_recv() {
                let sf_list = [radio_config.spreading_factor.value()];
                Python::with_gil(|py| {
                    match lora_receiver
                    .call_method(py, "recv_radio", (sf_list, extract_dev_id(d_id) ,timeout.map(|d| d.as_secs())), None)
                    .unwrap()
                    .extract(py)
                    {
                        Ok(v) => {
                            println!("Received!");
                            let _ = sender.send((true, v));
                        }
                        Err(e) => {
                            println!("{e}");
                        }
                    };
                });
            }
            println!("Thread receiver died");
        });
        
        Self {
            sender_send,
            receiver_send,
        }
    }
}

#[async_trait]
impl LoRaWANCommunication for ColosseumCommunication {
    async fn send_uplink(
        &mut self,
        bytes: &[u8],
        src: Option<EUI64>,
        dest: Option<EUI64>,
    ) -> Result<(), CommunicationError> {
        let (send, recv) = oneshot::channel();
        let _ = self.sender_send.send(((bytes.to_vec(), src, dest), send)).await;
        match recv.await {
            Ok(r) => {
                if r {
                    Ok(())
                } else {
                    Err(CommunicationError::Radio(
                        "Unable to send message".to_string(),
                    ))
                }
            }
            Err(e) => {
                eprintln!("{e:?}");
                Err(CommunicationError::Radio(
                    "Cannot send command to radio thread".to_string(),
                ))
            }
        }
    }

    async fn receive_downlink(
        &mut self,
        d_id: Option<EUI64>,
        timeout: Option<Duration>,
    ) -> Result<HashMap<SpreadingFactor, LoRaPacket>, CommunicationError> {
        println!("Waiting for downlink!");
        let (send, recv) = oneshot::channel();
        let _ = self
            .receiver_send
            .send(((d_id, timeout), send))
            .await;

        match recv.await {
            Ok((res, buffers)) => {
                if res {
                    println!("Ended waiting! Received {} packets", buffers.len());
                    Ok(buffers
                        .into_iter()
                        .map(|(sf, p)| (SpreadingFactor::new(sf), p))
                        .collect())
                } else {
                    Err(CommunicationError::Radio(
                        "Error receiving packets".to_string(),
                    ))
                }
            }
            Err(e) => {
                eprintln!("{e:?}");
                Err(CommunicationError::Radio(
                    "Error sending command to radio thread".to_string(),
                ))
            }
        }
    }
}


pub struct MockCommunicator;

#[async_trait]
impl LoRaWANCommunication for MockCommunicator {
    async fn send_uplink(
        &mut self,
        bytes: &[u8],
        _src: Option<EUI64>,
        _dest: Option<EUI64>,
    ) -> Result<(), CommunicationError> {
        println!("{}", PrettyHexSlice(bytes));
        Ok(())
    }
    
    async fn receive_downlink(
        &mut self,
        _d_id: Option<EUI64>,
        _timeout: Option<Duration>,
    ) -> Result<HashMap<SpreadingFactor, LoRaPacket>, CommunicationError> {
        Ok(HashMap::from([(SpreadingFactor::new(7), LoRaPacket {
            payload: Vec::from([0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]),
            ..Default::default()
        })]))
    }  
}