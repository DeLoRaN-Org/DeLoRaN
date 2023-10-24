#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Location {
    /// Latitude.
    #[prost(double, tag="1")]
    pub latitude: f64,
    /// Longitude.
    #[prost(double, tag="2")]
    pub longitude: f64,
    /// Altitude.
    #[prost(double, tag="3")]
    pub altitude: f64,
    /// Location source.
    #[prost(enumeration="LocationSource", tag="4")]
    pub source: i32,
    /// Accuracy.
    #[prost(float, tag="5")]
    pub accuracy: f32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct KeyEnvelope {
    /// KEK label.
    #[prost(string, tag="1")]
    pub kek_label: std::string::String,
    /// AES key (when the kek_label is set, this value must first be decrypted).
    #[prost(bytes, tag="2")]
    pub aes_key: std::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Metric {
    /// Name.
    #[prost(string, tag="1")]
    pub name: std::string::String,
    /// Timestamps.
    #[prost(message, repeated, tag="2")]
    pub timestamps: ::std::vec::Vec<::prost_types::Timestamp>,
    /// Datasets.
    #[prost(message, repeated, tag="3")]
    pub datasets: ::std::vec::Vec<MetricDataset>,
    /// Kind.
    #[prost(enumeration="MetricKind", tag="4")]
    pub kind: i32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MetricDataset {
    /// Label.
    #[prost(string, tag="1")]
    pub label: std::string::String,
    /// Data.
    /// Each value index corresponds with the same timestamp index of the Metric.
    #[prost(float, repeated, tag="2")]
    pub data: ::std::vec::Vec<f32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Modulation {
    /// LoRa
    Lora = 0,
    /// FSK
    Fsk = 1,
    /// LR-FHSS
    LrFhss = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Region {
    /// EU868
    Eu868 = 0,
    /// US915
    Us915 = 2,
    /// CN779
    Cn779 = 3,
    /// EU433
    Eu433 = 4,
    /// AU915
    Au915 = 5,
    /// CN470
    Cn470 = 6,
    /// AS923
    As923 = 7,
    /// AS923 with -1.80 MHz frequency offset
    As9232 = 12,
    /// AS923 with -6.60 MHz frequency offset
    As9233 = 13,
    /// (AS923 with -5.90 MHz frequency offset).
    As9234 = 14,
    /// KR920
    Kr920 = 8,
    /// IN865
    In865 = 9,
    /// RU864
    Ru864 = 10,
    /// ISM2400 (LoRaWAN 2.4 GHz)
    Ism2400 = 11,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum MType {
    /// JoinRequest.
    JoinRequest = 0,
    /// JoinAccept.
    JoinAccept = 1,
    /// UnconfirmedDataUp.
    UnconfirmedDataUp = 2,
    /// UnconfirmedDataDown.
    UnconfirmedDataDown = 3,
    /// ConfirmedDataUp.
    ConfirmedDataUp = 4,
    /// ConfirmedDataDown.
    ConfirmedDataDown = 5,
    /// RejoinRequest.
    RejoinRequest = 6,
    /// Proprietary.
    Proprietary = 7,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum MacVersion {
    Lorawan100 = 0,
    Lorawan101 = 1,
    Lorawan102 = 2,
    Lorawan103 = 3,
    Lorawan104 = 4,
    Lorawan110 = 5,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum RegParamsRevision {
    A = 0,
    B = 1,
    Rp002100 = 2,
    Rp002101 = 3,
    Rp002102 = 4,
    Rp002103 = 5,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum LocationSource {
    /// Unknown.
    Unknown = 0,
    /// GPS.
    Gps = 1,
    /// Manually configured.
    Config = 2,
    /// Geo resolver (TDOA).
    GeoResolverTdoa = 3,
    /// Geo resolver (RSSI).
    GeoResolverRssi = 4,
    /// Geo resolver (GNSS).
    GeoResolverGnss = 5,
    /// Geo resolver (WIFI).
    GeoResolverWifi = 6,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Aggregation {
    /// Hour.
    Hour = 0,
    /// Day.
    Day = 1,
    /// Month.
    Month = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum MetricKind {
    /// Incrementing counters that never decrease (these are not reset on each reading).
    Counter = 0,
    /// Counters that do get reset upon reading.
    Absolute = 1,
    /// E.g. a temperature value.
    Gauge = 2,
}
