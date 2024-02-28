use std::net::IpAddr;

use lorawan::{device::Device, physical_parameters::{CodeRate, DataRate, SpreadingFactor}, regional_parameters::region::Region};
use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TcpDeviceConfig {
    pub addr: String,
    pub port: u16
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct RadioDeviceConfig {
    pub region: Region,
    pub spreading_factor: SpreadingFactor,
    pub data_rate: DataRate,
    pub code_rate: CodeRate,
    pub rx_gain: u8,
    pub tx_gain: u8,
    pub bandwidth: f32,
    pub rx_freq: f32,
    pub tx_freq: f32,
    pub sample_rate: f32,
    pub rx_chan_id: u8,
    pub tx_chan_id: u8,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ColosseumDeviceConfig {
    pub radio_config: RadioDeviceConfig,
    pub address: IpAddr,
    pub sdr_code: String,
    pub dev_id: u16
}

pub struct MockDeviceConfig {
    pub radio_config: RadioDeviceConfig,
    pub address: IpAddr,
}


#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum DeviceConfigType {
    TCP(TcpDeviceConfig),
    RADIO(RadioDeviceConfig),
    COLOSSEUM(ColosseumDeviceConfig),
    MOCK
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DeviceConfig {
    pub dtype: DeviceConfigType,
    pub configuration: Device,
}