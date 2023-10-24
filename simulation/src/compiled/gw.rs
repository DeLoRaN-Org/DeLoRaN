#![allow(clippy::enum_variant_names)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Modulation {
    #[prost(oneof="modulation::Parameters", tags="3, 4, 5")]
    pub parameters: ::std::option::Option<modulation::Parameters>,
}
pub mod modulation {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Parameters {
        /// LoRa modulation information.
        #[prost(message, tag="3")]
        Lora(super::LoraModulationInfo),
        /// FSK modulation information.
        #[prost(message, tag="4")]
        Fsk(super::FskModulationInfo),
        /// LR-FHSS modulation information.
        #[prost(message, tag="5")]
        LrFhss(super::LrFhssModulationInfo),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UplinkTxInfoLegacy {
    /// Frequency (Hz).
    #[prost(uint32, tag="1")]
    pub frequency: u32,
    /// Modulation.
    #[prost(enumeration="super::common::Modulation", tag="2")]
    pub modulation: i32,
    #[prost(oneof="uplink_tx_info_legacy::ModulationInfo", tags="3, 4, 5")]
    pub modulation_info: ::std::option::Option<uplink_tx_info_legacy::ModulationInfo>,
}
pub mod uplink_tx_info_legacy {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum ModulationInfo {
        /// LoRa modulation information.
        #[prost(message, tag="3")]
        LoraModulationInfo(super::LoraModulationInfo),
        /// FSK modulation information.
        #[prost(message, tag="4")]
        FskModulationInfo(super::FskModulationInfo),
        /// LR-FHSS modulation information.
        #[prost(message, tag="5")]
        LrFhssModulationInfo(super::LrFhssModulationInfo),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UplinkTxInfo {
    /// Frequency (Hz).
    #[prost(uint32, tag="1")]
    pub frequency: u32,
    /// Modulation.
    #[prost(message, optional, tag="2")]
    pub modulation: ::std::option::Option<Modulation>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LoraModulationInfo {
    /// Bandwidth.
    #[prost(uint32, tag="1")]
    pub bandwidth: u32,
    /// Speading-factor.
    #[prost(uint32, tag="2")]
    pub spreading_factor: u32,
    /// Code-rate.
    #[prost(string, tag="3")]
    pub code_rate_legacy: std::string::String,
    /// Code-rate.
    #[prost(enumeration="CodeRate", tag="5")]
    pub code_rate: i32,
    /// Polarization inversion.
    #[prost(bool, tag="4")]
    pub polarization_inversion: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FskModulationInfo {
    /// Frequency deviation.
    #[prost(uint32, tag="1")]
    pub frequency_deviation: u32,
    /// FSK datarate (bits / sec).
    #[prost(uint32, tag="2")]
    pub datarate: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LrFhssModulationInfo {
    /// Operating channel width (OCW) in Hz.
    #[prost(uint32, tag="1")]
    pub operating_channel_width: u32,
    /// Code-rate.
    /// Deprecated: use code_rate.
    #[prost(string, tag="2")]
    pub code_rate_legacy: std::string::String,
    /// Code-rate.
    #[prost(enumeration="CodeRate", tag="4")]
    pub code_rate: i32,
    /// Hopping grid number of steps.
    #[prost(uint32, tag="3")]
    pub grid_steps: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EncryptedFineTimestamp {
    /// AES key index used for encrypting the fine timestamp.
    #[prost(uint32, tag="1")]
    pub aes_key_index: u32,
    /// Encrypted 'main' fine-timestamp (ns precision part of the timestamp).
    #[prost(bytes, tag="2")]
    pub encrypted_ns: std::vec::Vec<u8>,
    /// FPGA ID.
    #[prost(bytes, tag="3")]
    pub fpga_id: std::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PlainFineTimestamp {
    /// Full timestamp.
    #[prost(message, optional, tag="1")]
    pub time: ::std::option::Option<::prost_types::Timestamp>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GatewayStats {
    /// Gateway ID.
    /// Deprecated: use gateway_id.
    #[prost(bytes, tag="1")]
    pub gateway_id_legacy: std::vec::Vec<u8>,
    /// Gateway ID.
    #[prost(string, tag="17")]
    pub gateway_id: std::string::String,
    /// Gateway time.
    #[prost(message, optional, tag="2")]
    pub time: ::std::option::Option<::prost_types::Timestamp>,
    /// Gateway location.
    #[prost(message, optional, tag="3")]
    pub location: ::std::option::Option<super::common::Location>,
    /// Gateway configuration version (this maps to the config_version sent
    /// by ChirpStack to the gateway).
    #[prost(string, tag="4")]
    pub config_version: std::string::String,
    /// Number of radio packets received.
    #[prost(uint32, tag="5")]
    pub rx_packets_received: u32,
    /// Number of radio packets received with valid PHY CRC.
    #[prost(uint32, tag="6")]
    pub rx_packets_received_ok: u32,
    /// Number of downlink packets received for transmission.
    #[prost(uint32, tag="7")]
    pub tx_packets_received: u32,
    /// Number of downlink packets emitted.
    #[prost(uint32, tag="8")]
    pub tx_packets_emitted: u32,
    /// Additional gateway meta-data.
    #[prost(map="string, string", tag="10")]
    pub metadata: ::std::collections::HashMap<std::string::String, std::string::String>,
    /// Tx packets per frequency.
    #[prost(map="uint32, uint32", tag="12")]
    pub tx_packets_per_frequency: ::std::collections::HashMap<u32, u32>,
    /// Rx packets per frequency.
    #[prost(map="uint32, uint32", tag="13")]
    pub rx_packets_per_frequency: ::std::collections::HashMap<u32, u32>,
    /// Tx packets per modulation parameters.
    #[prost(message, repeated, tag="14")]
    pub tx_packets_per_modulation: ::std::vec::Vec<PerModulationCount>,
    /// Rx packets per modulation parameters.
    #[prost(message, repeated, tag="15")]
    pub rx_packets_per_modulation: ::std::vec::Vec<PerModulationCount>,
    /// Tx packets per status.
    #[prost(map="string, uint32", tag="16")]
    pub tx_packets_per_status: ::std::collections::HashMap<std::string::String, u32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PerModulationCount {
    /// Modulation.
    #[prost(message, optional, tag="1")]
    pub modulation: ::std::option::Option<Modulation>,
    /// Count.
    #[prost(uint32, tag="2")]
    pub count: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UplinkRxInfoLegacy {
    /// Gateway ID.
    #[prost(bytes, tag="1")]
    pub gateway_id: std::vec::Vec<u8>,
    /// RX time (only set when the gateway has a GPS module).
    #[prost(message, optional, tag="2")]
    pub time: ::std::option::Option<::prost_types::Timestamp>,
    /// RX time since GPS epoch (only set when the gateway has a GPS module).
    #[prost(message, optional, tag="3")]
    pub time_since_gps_epoch: ::std::option::Option<::prost_types::Duration>,
    /// RSSI.
    #[prost(int32, tag="5")]
    pub rssi: i32,
    /// LoRa SNR.
    #[prost(double, tag="6")]
    pub lora_snr: f64,
    /// Channel.
    #[prost(uint32, tag="7")]
    pub channel: u32,
    /// RF Chain.
    #[prost(uint32, tag="8")]
    pub rf_chain: u32,
    /// Board.
    #[prost(uint32, tag="9")]
    pub board: u32,
    /// Antenna.
    #[prost(uint32, tag="10")]
    pub antenna: u32,
    /// Location.
    #[prost(message, optional, tag="11")]
    pub location: ::std::option::Option<super::common::Location>,
    /// Fine-timestamp type.
    #[prost(enumeration="FineTimestampType", tag="12")]
    pub fine_timestamp_type: i32,
    /// Gateway specific context.
    #[prost(bytes, tag="15")]
    pub context: std::vec::Vec<u8>,
    /// Uplink ID (UUID bytes).
    /// Unique and random ID which can be used to correlate the uplink across multiple logs.
    #[prost(bytes, tag="16")]
    pub uplink_id: std::vec::Vec<u8>,
    /// CRC status.
    #[prost(enumeration="CrcStatus", tag="17")]
    pub crc_status: i32,
    /// Optional meta-data map.
    #[prost(map="string, string", tag="18")]
    pub metadata: ::std::collections::HashMap<std::string::String, std::string::String>,
    /// Fine-timestamp data.
    #[prost(oneof="uplink_rx_info_legacy::FineTimestamp", tags="13, 14")]
    pub fine_timestamp: ::std::option::Option<uplink_rx_info_legacy::FineTimestamp>,
}
pub mod uplink_rx_info_legacy {
    /// Fine-timestamp data.
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum FineTimestamp {
        /// Encrypted fine-timestamp data.
        #[prost(message, tag="13")]
        EncryptedFineTimestamp(super::EncryptedFineTimestamp),
        /// Plain fine-timestamp data.
        #[prost(message, tag="14")]
        PlainFineTimestamp(super::PlainFineTimestamp),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UplinkRxInfo {
    /// Gateway ID.
    #[prost(string, tag="1")]
    pub gateway_id: std::string::String,
    /// Uplink ID.
    #[prost(uint32, tag="2")]
    pub uplink_id: u32,
    /// RX time (only set when the gateway has a GPS module).
    #[prost(message, optional, tag="3")]
    pub time: ::std::option::Option<::prost_types::Timestamp>,
    /// RX time since GPS epoch (only set when the gateway has a GPS module).
    #[prost(message, optional, tag="4")]
    pub time_since_gps_epoch: ::std::option::Option<::prost_types::Duration>,
    /// Fine-timestamp.
    /// This timestamp can be used for TDOA based geolocation.
    #[prost(message, optional, tag="5")]
    pub fine_time_since_gps_epoch: ::std::option::Option<::prost_types::Duration>,
    /// RSSI.
    #[prost(int32, tag="6")]
    pub rssi: i32,
    /// SNR.
    /// Note: only available for LoRa modulation.
    #[prost(float, tag="7")]
    pub snr: f32,
    /// Channel.
    #[prost(uint32, tag="8")]
    pub channel: u32,
    /// RF chain.
    #[prost(uint32, tag="9")]
    pub rf_chain: u32,
    /// Board.
    #[prost(uint32, tag="10")]
    pub board: u32,
    /// Antenna.
    #[prost(uint32, tag="11")]
    pub antenna: u32,
    /// Location.
    #[prost(message, optional, tag="12")]
    pub location: ::std::option::Option<super::common::Location>,
    /// Gateway specific context.
    /// This value must be returned to the gateway on (Class-A) downlink.
    #[prost(bytes, tag="13")]
    pub context: std::vec::Vec<u8>,
    /// Additional gateway meta-data.
    #[prost(map="string, string", tag="15")]
    pub metadata: ::std::collections::HashMap<std::string::String, std::string::String>,
    /// CRC status.
    #[prost(enumeration="CrcStatus", tag="16")]
    pub crc_status: i32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DownlinkTxInfoLegacy {
    /// Gateway ID.
    /// Deprecated: replaced by gateway_id in DownlinkFrame.
    #[prost(bytes, tag="1")]
    pub gateway_id: std::vec::Vec<u8>,
    /// TX frequency (in Hz).
    #[prost(uint32, tag="5")]
    pub frequency: u32,
    /// TX power (in dBm).
    #[prost(int32, tag="6")]
    pub power: i32,
    /// Modulation.
    #[prost(enumeration="super::common::Modulation", tag="7")]
    pub modulation: i32,
    /// The board identifier for emitting the frame.
    #[prost(uint32, tag="10")]
    pub board: u32,
    /// The antenna identifier for emitting the frame.
    #[prost(uint32, tag="11")]
    pub antenna: u32,
    /// Timing defines the downlink timing to use.
    #[prost(enumeration="DownlinkTiming", tag="12")]
    pub timing: i32,
    /// Gateway specific context.
    /// In case of a Class-A downlink, this contains a copy of the uplink context.
    #[prost(bytes, tag="16")]
    pub context: std::vec::Vec<u8>,
    #[prost(oneof="downlink_tx_info_legacy::ModulationInfo", tags="8, 9")]
    pub modulation_info: ::std::option::Option<downlink_tx_info_legacy::ModulationInfo>,
    #[prost(oneof="downlink_tx_info_legacy::TimingInfo", tags="13, 14, 15")]
    pub timing_info: ::std::option::Option<downlink_tx_info_legacy::TimingInfo>,
}
pub mod downlink_tx_info_legacy {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum ModulationInfo {
        /// LoRa modulation information.
        #[prost(message, tag="8")]
        LoraModulationInfo(super::LoraModulationInfo),
        /// FSK modulation information.
        #[prost(message, tag="9")]
        FskModulationInfo(super::FskModulationInfo),
    }
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum TimingInfo {
        /// Immediately timing information.
        #[prost(message, tag="13")]
        ImmediatelyTimingInfo(super::ImmediatelyTimingInfo),
        /// Context based delay timing information.
        #[prost(message, tag="14")]
        DelayTimingInfo(super::DelayTimingInfo),
        /// GPS Epoch timing information.
        #[prost(message, tag="15")]
        GpsEpochTimingInfo(super::GpsEpochTimingInfo),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DownlinkTxInfo {
    /// TX frequency (in Hz).
    #[prost(uint32, tag="1")]
    pub frequency: u32,
    /// TX power (in dBm).
    #[prost(int32, tag="2")]
    pub power: i32,
    /// Modulation.
    #[prost(message, optional, tag="3")]
    pub modulation: ::std::option::Option<Modulation>,
    /// The board identifier for emitting the frame.
    #[prost(uint32, tag="4")]
    pub board: u32,
    /// The antenna identifier for emitting the frame.
    #[prost(uint32, tag="5")]
    pub antenna: u32,
    /// Timing.
    #[prost(message, optional, tag="6")]
    pub timing: ::std::option::Option<Timing>,
    /// Gateway specific context.
    /// In case of a Class-A downlink, this contains a copy of the uplink context.
    #[prost(bytes, tag="7")]
    pub context: std::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Timing {
    #[prost(oneof="timing::Parameters", tags="1, 2, 3")]
    pub parameters: ::std::option::Option<timing::Parameters>,
}
pub mod timing {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Parameters {
        /// Immediately timing information.
        #[prost(message, tag="1")]
        Immediately(super::ImmediatelyTimingInfo),
        /// Context based delay timing information.
        #[prost(message, tag="2")]
        Delay(super::DelayTimingInfo),
        /// GPS Epoch timing information.
        #[prost(message, tag="3")]
        GpsEpoch(super::GpsEpochTimingInfo),
    }
}
/// Not implemented yet.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ImmediatelyTimingInfo {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DelayTimingInfo {
    /// Delay (duration).
    /// The delay will be added to the gateway internal timing, provided by the context object.
    #[prost(message, optional, tag="1")]
    pub delay: ::std::option::Option<::prost_types::Duration>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GpsEpochTimingInfo {
    /// Duration since GPS Epoch.
    #[prost(message, optional, tag="1")]
    pub time_since_gps_epoch: ::std::option::Option<::prost_types::Duration>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UplinkFrame {
    /// PHYPayload.
    #[prost(bytes, tag="1")]
    pub phy_payload: std::vec::Vec<u8>,
    /// TX meta-data (deprecated).
    #[prost(message, optional, tag="2")]
    pub tx_info_legacy: ::std::option::Option<UplinkTxInfoLegacy>,
    /// RX meta-data (deprecated).
    #[prost(message, optional, tag="3")]
    pub rx_info_legacy: ::std::option::Option<UplinkRxInfoLegacy>,
    /// Tx meta-data.
    #[prost(message, optional, tag="4")]
    pub tx_info: ::std::option::Option<UplinkTxInfo>,
    /// Rx meta-data.
    #[prost(message, optional, tag="5")]
    pub rx_info: ::std::option::Option<UplinkRxInfo>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UplinkFrameSet {
    /// PHYPayload.
    #[prost(bytes, tag="1")]
    pub phy_payload: std::vec::Vec<u8>,
    /// TX meta-data.
    #[prost(message, optional, tag="2")]
    pub tx_info: ::std::option::Option<UplinkTxInfo>,
    /// RX meta-data set.
    #[prost(message, repeated, tag="3")]
    pub rx_info: ::std::vec::Vec<UplinkRxInfo>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DownlinkFrame {
    /// Downlink ID.
    #[prost(uint32, tag="3")]
    pub downlink_id: u32,
    /// Downlink ID (UUID).
    /// Deprecated: use downlink_id.
    #[prost(bytes, tag="4")]
    pub downlink_id_legacy: std::vec::Vec<u8>,
    /// Downlink frame items.
    /// This makes it possible to send multiple downlink opportunities to the
    /// gateway at once (e.g. RX1 and RX2 in LoRaWAN). The first item has the
    /// highest priority, the last the lowest. The gateway will emit at most
    /// one item.
    #[prost(message, repeated, tag="5")]
    pub items: ::std::vec::Vec<DownlinkFrameItem>,
    /// Gateway ID.
    /// Deprecated: use gateway_id
    #[prost(bytes, tag="6")]
    pub gateway_id_legacy: std::vec::Vec<u8>,
    /// Gateway ID.
    #[prost(string, tag="7")]
    pub gateway_id: std::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DownlinkFrameItem {
    /// PHYPayload.
    #[prost(bytes, tag="1")]
    pub phy_payload: std::vec::Vec<u8>,
    /// TX meta-data (deprecated).
    #[prost(message, optional, tag="2")]
    pub tx_info_legacy: ::std::option::Option<DownlinkTxInfoLegacy>,
    /// Tx meta-data.
    #[prost(message, optional, tag="3")]
    pub tx_info: ::std::option::Option<DownlinkTxInfo>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DownlinkTxAck {
    /// Gateway ID (deprecated).
    #[prost(bytes, tag="1")]
    pub gateway_id_legacy: std::vec::Vec<u8>,
    /// Gateway ID.
    #[prost(string, tag="6")]
    pub gateway_id: std::string::String,
    /// Downlink ID.
    #[prost(uint32, tag="2")]
    pub downlink_id: u32,
    /// Downlink ID (deprecated).
    #[prost(bytes, tag="4")]
    pub downlink_id_legacy: std::vec::Vec<u8>,
    /// Downlink frame items.
    /// This list has the same length as the request and indicates which
    /// downlink frame has been emitted of the requested list (or why it failed).
    /// Note that at most one item has a positive acknowledgement.
    #[prost(message, repeated, tag="5")]
    pub items: ::std::vec::Vec<DownlinkTxAckItem>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DownlinkTxAckItem {
    /// The Ack status of this item.
    #[prost(enumeration="TxAckStatus", tag="1")]
    pub status: i32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GatewayConfiguration {
    /// Gateway ID.
    /// Deprecated: use gateway_id.
    #[prost(bytes, tag="1")]
    pub gateway_id_legacy: std::vec::Vec<u8>,
    /// Gateway ID.
    #[prost(string, tag="5")]
    pub gateway_id: std::string::String,
    /// Configuration version.
    #[prost(string, tag="2")]
    pub version: std::string::String,
    /// Channels.
    #[prost(message, repeated, tag="3")]
    pub channels: ::std::vec::Vec<ChannelConfiguration>,
    /// Stats interval.
    #[prost(message, optional, tag="4")]
    pub stats_interval: ::std::option::Option<::prost_types::Duration>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ChannelConfiguration {
    /// Frequency (Hz).
    #[prost(uint32, tag="1")]
    pub frequency: u32,
    /// Modulation (deprecated).
    #[prost(enumeration="super::common::Modulation", tag="2")]
    pub modulation_legacy: i32,
    /// Board index.
    #[prost(uint32, tag="5")]
    pub board: u32,
    /// Demodulator index (of the given board).
    #[prost(uint32, tag="6")]
    pub demodulator: u32,
    #[prost(oneof="channel_configuration::ModulationConfig", tags="3, 4")]
    pub modulation_config: ::std::option::Option<channel_configuration::ModulationConfig>,
}
pub mod channel_configuration {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum ModulationConfig {
        /// LoRa modulation config.
        #[prost(message, tag="3")]
        LoraModulationConfig(super::LoraModulationConfig),
        /// FSK modulation config.
        #[prost(message, tag="4")]
        FskModulationConfig(super::FskModulationConfig),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LoraModulationConfig {
    /// Bandwidth (kHz).
    /// Deprecated: use bandwidth.
    #[prost(uint32, tag="1")]
    pub bandwidth_legacy: u32,
    /// Bandwidth (Hz).
    #[prost(uint32, tag="3")]
    pub bandwidth: u32,
    /// Spreading-factors.
    #[prost(uint32, repeated, tag="2")]
    pub spreading_factors: ::std::vec::Vec<u32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FskModulationConfig {
    /// Bandwidth (kHz).
    /// Deprecated: use bandwidth.
    #[prost(uint32, tag="1")]
    pub bandwidth_legacy: u32,
    /// Bandwidth (Hz).
    #[prost(uint32, tag="3")]
    pub bandwidth: u32,
    /// Bitrate.
    #[prost(uint32, tag="2")]
    pub bitrate: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GatewayCommandExecRequest {
    /// Gateway ID.
    /// Deprecated: use gateway_id.
    #[prost(bytes, tag="1")]
    pub gateway_id_legacy: std::vec::Vec<u8>,
    /// Gateway ID.
    #[prost(string, tag="6")]
    pub gateway_id: std::string::String,
    /// Command to execute.
    /// This command must be pre-configured in the LoRa Gateway Bridge configuration.
    #[prost(string, tag="2")]
    pub command: std::string::String,
    /// Execution request ID.
    /// The same will be returned when the execution of the command has
    /// completed.
    #[prost(uint32, tag="7")]
    pub exec_id: u32,
    /// Standard input.
    #[prost(bytes, tag="4")]
    pub stdin: std::vec::Vec<u8>,
    /// Environment variables.
    #[prost(map="string, string", tag="5")]
    pub environment: ::std::collections::HashMap<std::string::String, std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GatewayCommandExecResponse {
    /// Gateway ID.
    /// Deprecated: use gateway_id.
    #[prost(bytes, tag="1")]
    pub gateway_id_legacy: std::vec::Vec<u8>,
    /// Gateway ID.
    #[prost(string, tag="6")]
    pub gateway_id: std::string::String,
    /// Execution request ID.
    #[prost(uint32, tag="7")]
    pub exec_id: u32,
    /// Standard output.
    #[prost(bytes, tag="3")]
    pub stdout: std::vec::Vec<u8>,
    /// Standard error.
    #[prost(bytes, tag="4")]
    pub stderr: std::vec::Vec<u8>,
    /// Error message.
    #[prost(string, tag="5")]
    pub error: std::string::String,
}
/// RawPacketForwarderEvent contains a raw packet-forwarder event.
/// It can be used to access packet-forwarder features that are not (fully)
/// integrated with the ChirpStack Gateway Bridge.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RawPacketForwarderEvent {
    /// Gateway ID.
    /// Deprecated: use gateway_id.
    #[prost(bytes, tag="1")]
    pub gateway_id_legacy: std::vec::Vec<u8>,
    /// Gateway ID.
    #[prost(string, tag="4")]
    pub gateway_id: std::string::String,
    /// Payload contains the raw payload.
    #[prost(bytes, tag="3")]
    pub payload: std::vec::Vec<u8>,
}
/// RawPacketForwarderEvent contains a raw packet-forwarder command.
/// It can be used to access packet-forwarder features that are not (fully)
/// integrated with the ChirpStack Gateway Bridge.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RawPacketForwarderCommand {
    /// Gateway ID.
    /// Deprecated: use gateway_id.
    #[prost(bytes, tag="1")]
    pub gateway_id_legacy: std::vec::Vec<u8>,
    /// Gateway ID.
    #[prost(string, tag="4")]
    pub gateway_id: std::string::String,
    /// Payload contains the raw payload.
    #[prost(bytes, tag="3")]
    pub payload: std::vec::Vec<u8>,
}
/// ConnState contains the connection state of a gateway.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ConnState {
    /// Gateway ID.
    /// Deprecated: use gateway_id.
    #[prost(bytes, tag="1")]
    pub gateway_id_legacy: std::vec::Vec<u8>,
    /// Gateway ID.
    #[prost(string, tag="3")]
    pub gateway_id: std::string::String,
    #[prost(enumeration="conn_state::State", tag="2")]
    pub state: i32,
}
pub mod conn_state {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum State {
        Offline = 0,
        Online = 1,
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum CodeRate {
    CrUndefined = 0,
    /// LoRa
    Cr45 = 1,
    Cr46 = 2,
    Cr47 = 3,
    Cr48 = 4,
    /// LR-FHSS
    Cr38 = 5,
    Cr26 = 6,
    Cr14 = 7,
    Cr16 = 8,
    Cr56 = 9,
    /// LoRa 2.4 gHz
    CrLi45 = 10,
    CrLi46 = 11,
    CrLi48 = 12,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum DownlinkTiming {
    /// Send the downlink immediately.
    Immediately = 0,
    /// Send downlink at the given delay (based on provided context).
    Delay = 1,
    /// Send at given GPS epoch value.
    GpsEpoch = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum FineTimestampType {
    /// No fine-timestamp available.
    None = 0,
    /// Encrypted fine-timestamp.
    Encrypted = 1,
    /// Plain fine-timestamp.
    Plain = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum CrcStatus {
    /// No CRC.
    NoCrc = 0,
    /// Bad CRC.
    BadCrc = 1,
    /// CRC OK.
    CrcOk = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum TxAckStatus {
    /// Ignored (when a previous item was already emitted).
    Ignored = 0,
    /// Packet has been programmed for downlink.
    Ok = 1,
    /// Rejected because it was already too late to program this packet for downlink.
    TooLate = 2,
    /// Rejected because downlink packet timestamp is too much in advance.
    TooEarly = 3,
    /// Rejected because there was already a packet programmed in requested timeframe.
    CollisionPacket = 4,
    /// Rejected because there was already a beacon planned in requested timeframe.
    CollisionBeacon = 5,
    /// Rejected because requested frequency is not supported by TX RF chain.
    TxFreq = 6,
    /// Rejected because requested power is not supported by gateway.
    TxPower = 7,
    /// Rejected because GPS is unlocked, so GPS timestamp cannot be used.
    GpsUnlocked = 8,
    /// Downlink queue is full.
    QueueFull = 9,
    /// Internal error.
    InternalError = 10,
}
