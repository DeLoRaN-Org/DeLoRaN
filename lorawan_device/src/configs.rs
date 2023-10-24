use std::net::IpAddr;

use lorawan::{regional_parameters::region::Region, physical_parameters::{SpreadingFactor, DataRate}, device::Device};
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
    pub rx_gain: u8,
    pub tx_gain: u8,
    pub bandwidth: u32,
    pub rx_freq: f32,
    pub tx_freq: f32,
    pub sample_rate: f32,
    pub rx_chan_id: u8,
    pub tx_chan_id: u8,
    pub dev_id: u8
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct ColosseumDeviceConfig {
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
    pub configuration: Device,
    pub dtype: DeviceConfigType,
}