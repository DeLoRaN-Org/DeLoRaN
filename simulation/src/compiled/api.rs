#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UplinkFrameLog {
    /// PHYPayload.
    #[prost(bytes, tag="1")]
    pub phy_payload: std::vec::Vec<u8>,
    /// TX meta-data.
    #[prost(message, optional, tag="2")]
    pub tx_info: ::std::option::Option<super::gw::UplinkTxInfo>,
    /// RX meta-data.
    #[prost(message, repeated, tag="3")]
    pub rx_info: ::std::vec::Vec<super::gw::UplinkRxInfo>,
    /// Message type.
    #[prost(enumeration="super::common::MType", tag="4")]
    pub m_type: i32,
    /// Device address (optional).
    #[prost(string, tag="5")]
    pub dev_addr: std::string::String,
    /// Device EUI (optional).
    #[prost(string, tag="6")]
    pub dev_eui: std::string::String,
    /// Time.
    #[prost(message, optional, tag="7")]
    pub time: ::std::option::Option<::prost_types::Timestamp>,
    /// Plaintext mac-commands.
    #[prost(bool, tag="8")]
    pub plaintext_mac_commands: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DownlinkFrameLog {
    /// Time.
    #[prost(message, optional, tag="1")]
    pub time: ::std::option::Option<::prost_types::Timestamp>,
    /// PHYPayload.
    #[prost(bytes, tag="2")]
    pub phy_payload: std::vec::Vec<u8>,
    /// TX meta-data.
    #[prost(message, optional, tag="3")]
    pub tx_info: ::std::option::Option<super::gw::DownlinkTxInfo>,
    /// Downlink ID.
    #[prost(uint32, tag="4")]
    pub downlink_id: u32,
    /// Gateway ID (EUI64).
    #[prost(string, tag="5")]
    pub gateway_id: std::string::String,
    /// Message type.
    #[prost(enumeration="super::common::MType", tag="6")]
    pub m_type: i32,
    /// Device address (optional).
    #[prost(string, tag="7")]
    pub dev_addr: std::string::String,
    /// Device EUI (optional).
    #[prost(string, tag="8")]
    pub dev_eui: std::string::String,
    /// Plaintext mac-commands.
    #[prost(bool, tag="9")]
    pub plaintext_mac_commands: bool,
}
