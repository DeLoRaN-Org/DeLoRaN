use crate::{utils::{traits::ToBytes, errors::LoRaWANError}, device::DeviceClass};


///Commands sent by the end device
#[derive(Debug, PartialEq, Eq)]
pub enum EDMacCommands {
    /// 7..4 -> RFU, 3..0 -> Minor version ( -> Values: 0 -> RFU, 1 -> Lorawan x.1, -> 2..15 -> RFU)
    ResetInd(u8),
    LinkCheckReq,
    ///7..3 -> RFU, 2 -> power ack, 1 data rate ack, 0 channel mask ack ||| if one of the acks is 0 everything was ignored
    LinkADRAns {
        power_ack: bool,
        data_rate_ack: bool,
        channel_mask_ack: bool,
    },
    DutyCycleAns,
    ///7..3 -> RFU, 2 RX1DRoffsetACK, 1 RX2DataRateAck, 0 ChannelAck
    RXParamSetupAns {
        rx1_dr_offset_ack: bool,
        rx2_data_rate_ack: bool,
        channel_ack: bool
    },
    DevStatusAns {
        ///0 -> external power source, 1..254 battery level, 255 unable to check battery level
        battery: u8,
        ///7..6 -> RFU, 5..0 -> Margin ||| SNR rounded of the last DevStatusReq received
        margin: u8,
    },
    /// 7..2 -> RFU, 1 -> DataRateRange ACK, 0 -> ChannelFrequency ACK |||  if one of the acks is 0 everything was ignored
    NewChannelAns {
        data_range_ok: bool,
        channel_frequency_ok: bool
    },
    /// 7..2 -> RFU, 1 -> UplinkFrequency ACK, 0 -> ChannelFrequency ACK |||  if one of the acks is 0 everything was ignored
    RXTimingSetupAns,
    TxParamSetupAns,
    DlChannelAns {
        uplink_frequency_exists: bool,
        channel_frequency_ok: bool
    },
    ///7..4 -> RFU, 3..0 -> Minor ||| Minor value of 0 is RFU, 1 indicates LoRaWAN x.1, 2..15 is RFU
    RekeyInd(u8),
    ADRParamSetupAns,
    DeviceTimeReq,
    /// 7..1 -> RFU, 0 TimeACK
    RejoinParamSetupAns {
        time_ack: bool
    },

    //CLASS B MAC COMMANDS
    PingSlotInfoReq {
        periodicity: u8
    },
    PingSlotChannelAns { //TODO check if this is PingSlotChannelAns or PingSlotFreqAns
        data_rate_ok: bool,
        channel_frequency_ok: bool
    },
    BeaconFreqAns {
        beacon_freq_ok: bool,
    },

    //CLASS C MAC COMMANDS
    DeviceModeInd(DeviceClass),

    ///mac code 0x80 to 0xFF - custom payload
    Proprietary(u8, Vec<u8>),
}

impl EDMacCommands {
    pub fn from_bytes(bytes: &[u8]) -> Result<Vec<Self>, LoRaWANError> {
        if bytes.is_empty() {
            Err(LoRaWANError::InvalidBufferLength)
        }
        else {
            let mut mac_commands_vec = Vec::new();
            let mut bytes_iterator = bytes.iter();
            while let Some(&b) = bytes_iterator.next() {
                let new_command = match b {
                    0x01 => {
                        if let Some(ind) = bytes_iterator.next() {
                            EDMacCommands::ResetInd(ind & 0b00001111)
                        }
                        else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x02 => EDMacCommands::LinkCheckReq,
                    0x03 => {
                        if let Some(status) = bytes_iterator.next() {
                            EDMacCommands::LinkADRAns { 
                                power_ack:        *status & 0b00000100 > 0, 
                                data_rate_ack:    *status & 0b00000010 > 0, 
                                channel_mask_ack: *status & 0b00000001 > 0 
                            }
                        }
                        else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x04 => EDMacCommands::DutyCycleAns,
                    0x05 => {
                        if let Some(status) = bytes_iterator.next() {
                            EDMacCommands::RXParamSetupAns { 
                                rx1_dr_offset_ack: *status & 0b00000100 > 0, 
                                rx2_data_rate_ack: *status & 0b00000010 > 0, 
                                channel_ack:       *status & 0b00000001 > 0 
                            }
                        }
                        else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x06 => {
                        let battery = bytes_iterator.next();
                        let margin = bytes_iterator.next();
                        if let (Some(b), Some(m)) = (battery, margin) {
                            EDMacCommands::DevStatusAns { 
                                battery: *b, 
                                margin: *m & 0b00111111 
                            }
                        } else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x07 => {
                        if let Some(status) = bytes_iterator.next() {
                            EDMacCommands::NewChannelAns { 
                                data_range_ok:        *status & 0b00000010 > 0, 
                                channel_frequency_ok: *status & 0b00000001 > 0 
                            }
                        }
                        else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x08 => EDMacCommands::RXTimingSetupAns,
                    0x09 => EDMacCommands::TxParamSetupAns,
                    0x0A => {
                        if let Some(status) = bytes_iterator.next() {
                            EDMacCommands::DlChannelAns { 
                                uplink_frequency_exists:  *status & 0b00000010 > 0, 
                                channel_frequency_ok:     *status & 0b00000001 > 0 
                            }
                        }
                        else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x0B => {
                        if let Some(version) = bytes_iterator.next() {
                            EDMacCommands::RekeyInd(*version & 0b00001111)
                        }
                        else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x0C => EDMacCommands::ADRParamSetupAns,
                    0x0D => EDMacCommands::DeviceTimeReq,
                    0x0E => {
                        return Err(LoRaWANError::MalformedMACCommand)
                    }
                    0x0F => {
                        if let Some(ta) = bytes_iterator.next() {
                            EDMacCommands::RejoinParamSetupAns {
                                time_ack: *ta & 0b00000001 > 0,
                            }
                        }
                        else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x10 => {
                        if let Some(param) = bytes_iterator.next() {
                            EDMacCommands::PingSlotInfoReq {
                                periodicity: *param & 0b00000011,
                            }
                        }
                        else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x11 => {
                        if let Some(ans) = bytes_iterator.next() {
                            EDMacCommands::PingSlotChannelAns {
                                data_rate_ok: ans & 0b00000010 > 0,
                                channel_frequency_ok: ans & 0b00000001 > 0,
                            }
                        }
                        else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x13 => {
                        if let Some(status) = bytes_iterator.next() {
                            EDMacCommands::BeaconFreqAns { beacon_freq_ok: status & 0b00000001 > 0 }
                        }
                        else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x20 => {
                        if let Some(&class) = bytes_iterator.next() {
                            EDMacCommands::DeviceModeInd(DeviceClass::from_byte(class))
                        }
                        else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }

                    }
                    code if (0x80..=0xff).contains(&b) => {
                        let payload: Vec<u8> = bytes_iterator.by_ref().copied().collect();
                        EDMacCommands::Proprietary(code, payload)
                    }
                    _ => { //invalid code
                        return Err(LoRaWANError::MalformedMACCommand)
                    }
                };
                mac_commands_vec.push(new_command);
            }
            Ok(mac_commands_vec)
        }
    }
}

///Commands sent by the server
#[derive(Debug, PartialEq, Eq)]
pub enum NCMacCommands {
    /// same logic but regarding the server minor version and must be equal to the one sent by the device
    ResetConf(u8),
    LinkCheckAns {
        ///link margin in dB of the last LinkCheckReq received; 0dB == demodulation floor; 255 is reserved
        margin: u8,
        ///num of gateways which received the last LinkCheckReq
        gw_cnt: u8,
    },
    ///multiple blocks are allowed if contiguous
    LinkADRReq {
        ///data_rate_tx_power byte: 7..4 -> data rate, 3..0 -> TxPower |||| 0xF in these fields means that this value must be ignored
        data_rate: u8,
        tx_power: u8,
        
        ///encodes the channels for uplink, with LSB order, if the bit is set it means that that channel can be used
        ch_mask: u16,
        
        /// redundancy byte: 7 -> RFU, 6..4 -> ChMaskCntl, 3..0 -> NbTrans. If NbTrans is 0 it should be ignored ||||  ChMaskCntl is region specific and defined in PHY  // TODO
        ch_mask_cntl: u8,
        nb_trans: u8,
    },
    ///7..4 ->RFU, 3..0 -> MaxDCycle
    DutyCycleReq(u8),
    
    RXParamSetupReq {
        ///dl_settings byte: 7 -> RFU, 6..4 -> RX1DRoffset, 3..0 -> RX2DataRate
        rx1_dr_offset: u8,
        rx2_data_rate: u8,
        ///coded as defined in NewChannelReq command
        freq: u32,
    },
    DevStatusReq,
    NewChannelReq {
        ch_index: u8,
        freq: u32,
        /// 7..4 -> MaxDR, 3..0 -> MinDR
        max_dr: u8,
        min_dr: u8,
    },
    /// 7..4 -> RFU, 3..0 -> Delay (0 is 1s, 1 is 1s and 15 is 15s)
    RXTimingSetupReq(u8),
    /// 7..6 -> RFU, 5 -> DownlinkDwellTime*, 4 -> UplinkDwellTime*, 3..0 -> MaxEIRP***
    TxParamSetupReq {
        downlink_dwell_time: bool,
        uplink_dwell_time: bool,
        max_eirp: u8
    },
    DlChannelReq {
        ch_index: u8,
        freq: u32,
    },
    ///same format of RekeyInd but minor = 0 cannot be used
    RekeyConf(u8),
    /// 7..4 -> limit_exp, 3..0 delay_exp
    ADRParamSetupReq {
        limit_exp: u8,
        delay_exp: u8
    },
    DeviceTimeAns {
        epoch: u32,
        ///indicates steps of ½^8 seconds
        second_fraction: u8,
    },
    /// 15..14 -> RFU, 13..11 -> period, 10..8 -> max_retries, 7 RFU, 6..4 -> rejoin_type, 3..0 -> DR ****
    ForceRejoinReq {
        period: u8,
        max_retries: u8,
        rejoin_type: u8,
        dr: u8
    },
    /// 7..4 -> MaxTimeN, 3..0 -> MaxCountN *****
    RejoinParamSetupReq {
        max_time_n: u8,
        max_count_n: u8
    },

    //CLASS B MAC COMMANDS
    PingSlotInfoAns,
    PingSlotChannelReq {
        frequency: u32,
        data_rate: u8
    },  
    BeaconFreqReq {
        frequency: u32,
    },

    //CLASS C MAC COMMANDS
    DeviceModeConf(DeviceClass),

    ///mac code 0x80 to 0xFF - custom payload
    Proprietary(u8, Vec<u8>),
}

/*** Dwell Time ***                     ||| **** period -> delay between retransmissions = 32s x 2^period + rand(0,32)s        ||| ***** MaxCountN -> 0 to 15, the device has to send a rejoin after 2^(MaxCountN + 4) uplink
/         0 -> no limit                 |||      max_retries ->0 no retry, then 1 + max_retries values                         |||       MaxTimeN -> 0 to 15, the device has to send a rejoin after 2^(MaxTimeN + 10)s
/         1 -> 400ms                    |||      rejoin_type -> 0/1 rejoin request type 0, 2 rejoin request type 2, 3..7 RFU   |||
/ *** MaxEirp ***                       |||      DR -> the data rate to use to transmit the rejoin.                            |||       n.b. time limitations is optional
/         0  -> 8                       |||                                                                                    |||
/         1  -> 10                      |||                                                                                    |||
/         2  -> 12                      |||                                                                                    |||
/         3  -> 13                      |||
/         4  -> 14                      |||
/         5  -> 16                      |||
/         6  -> 18                      |||
/         7  -> 20                      |||
/         8  -> 21                      |||
/         9  -> 24                      |||
/        10  -> 26                      |||
/        11  -> 27                      |||
/        12  -> 29                      |||
/        13  -> 30                      |||
/        14  -> 33                      |||
/        15  -> 36                      |||*/
impl NCMacCommands {
    pub fn from_bytes(bytes: &[u8]) -> Result<Vec<Self>, LoRaWANError> {
        if bytes.is_empty() {
            Err(LoRaWANError::InvalidBufferLength)
        }
        else {
            let mut mac_commands_vec = Vec::new();
            let mut bytes_iterator = bytes.iter();
            while let Some(&b) = bytes_iterator.next() {
                let new_command = match b {
                    0x01 => {
                        if let Some(version) = bytes_iterator.next() {
                            NCMacCommands::ResetConf(version & 0b00001111)
                        }
                        else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x02 => {
                        let margin = bytes_iterator.next();
                        let gw_cnt = bytes_iterator.next();
                        if let (Some(&m), Some(&gc)) = (margin, gw_cnt) {
                            NCMacCommands::LinkCheckAns { margin: m, gw_cnt: gc }
                        } else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x03 => {
                        let data_rate_tx_power = bytes_iterator.next();
                        let ch_mask_byte_1 = bytes_iterator.next();
                        let ch_mask_byte_2 = bytes_iterator.next();
                        let redundancy = bytes_iterator.next();
                        if let (Some(&drtp), Some(&cmb1), Some(&cmb2), Some(&r)) = (data_rate_tx_power, ch_mask_byte_1, ch_mask_byte_2, redundancy) {
                            let data_rate = (drtp & 0b11110000) >> 4;
                            let tx_power = drtp & 0b00001111;
                            let ch_mask = u16::from_le_bytes([cmb1, cmb2]);
                            let ch_mask_cntl = (r & 0b01110000) >> 4;
                            let nb_trans = r & 0b00001111;
                            NCMacCommands::LinkADRReq { data_rate, tx_power, ch_mask, ch_mask_cntl, nb_trans}
                        } else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x04 => {
                        if let Some(b) = bytes_iterator.next() {
                            NCMacCommands::DutyCycleReq(b & 0b00001111)
                        }
                        else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x05 => {
                        let dl_settings = bytes_iterator.next();
                        let freq_b1 = bytes_iterator.next();
                        let freq_b2 = bytes_iterator.next();
                        let freq_b3 = bytes_iterator.next();
                        if let (Some(&ds), Some(&fb1), Some(&fb2), Some(&fb3)) = (dl_settings, freq_b1, freq_b2, freq_b3) {
                            let rx1_dr_offset = (ds & 0b01110000) >> 4;
                            let rx2_data_rate = ds & 0b00001111;
                            let freq = u32::from_le_bytes([fb1, fb2, fb3, 0]);
                            NCMacCommands::RXParamSetupReq { rx1_dr_offset, rx2_data_rate, freq }
                        } else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x06 => NCMacCommands::DevStatusReq,
                    0x07 => {
                        let ch_index = bytes_iterator.next();
                        let freq_b1 = bytes_iterator.next();
                        let freq_b2 = bytes_iterator.next();
                        let freq_b3 = bytes_iterator.next();
                        let data_range = bytes_iterator.next();
                        if let (Some(&ci), Some(&fb1), Some(&fb2), Some(&fb3), Some(&dr)) = (ch_index, freq_b1, freq_b2, freq_b3, data_range) {
                            let freq = u32::from_le_bytes([fb1, fb2, fb3, 0]);
                            let max_dr = (dr & 0b11110000) >> 4;
                            let min_dr = dr & 0b00001111;

                            NCMacCommands::NewChannelReq { ch_index: ci, freq, max_dr, min_dr }
                        } else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x08 => {
                        if let Some(settings) = bytes_iterator.next() {
                            NCMacCommands::RXTimingSetupReq(settings & 0b00001111)
                        }
                        else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x09 => {
                        if let Some(eirp_dwell_time) = bytes_iterator.next() {
                            let downlink_dwell_time = (eirp_dwell_time & 0b00100000) > 0; 
                            let uplink_dwell_time =   (eirp_dwell_time & 0b00010000) > 0; 
                            let max_eirp = eirp_dwell_time & 0b00001111;
                            NCMacCommands::TxParamSetupReq { downlink_dwell_time, uplink_dwell_time, max_eirp }
                        }
                        else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x0A => {
                        let ch_index = bytes_iterator.next();
                        let freq_b1 = bytes_iterator.next();
                        let freq_b2 = bytes_iterator.next();
                        let freq_b3 = bytes_iterator.next();
                        if let (Some(&ci), Some(&fb1), Some(&fb2), Some(&fb3)) = (ch_index, freq_b1, freq_b2, freq_b3) {
                            let freq = u32::from_le_bytes([fb1, fb2, fb3, 0]);
                            NCMacCommands::DlChannelReq { ch_index: ci, freq }
                        } else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x0B => {
                        if let Some(&version) = bytes_iterator.next() {
                            NCMacCommands::RekeyConf(version & 0b00001111)
                        }
                        else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x0C => {
                        if let Some(&adr_params) = bytes_iterator.next() {
                            let limit_exp = (adr_params & 0b11110000) >> 4;
                            let delay_exp = adr_params & 0b00001111;

                            NCMacCommands::ADRParamSetupReq { limit_exp, delay_exp }
                        }
                        else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x0D => {
                        let epoch_b0 = bytes_iterator.next();
                        let epoch_b1 = bytes_iterator.next();
                        let epoch_b2 = bytes_iterator.next();
                        let epoch_b3 = bytes_iterator.next();
                        let second_fraction = bytes_iterator.next();
                        if let (Some(&eb0), Some(&eb1), Some(&eb2), Some(&eb3), Some(&sf)) = (epoch_b0, epoch_b1, epoch_b2, epoch_b3, second_fraction) {
                            let epoch = u32::from_le_bytes([eb0, eb1, eb2, eb3]);
                            NCMacCommands::DeviceTimeAns { epoch, second_fraction: sf }
                        } else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x0E => {
                        let first_byte = bytes_iterator.next();
                        let second_byte = bytes_iterator.next();
                        if let (Some(&fb), Some(&sb)) = (first_byte, second_byte) {
                            let period = (fb & 0b00111000) >> 3;
                            let max_retries = fb & 0b00000111;

                            let rejoin_type = (sb & 0b01110000) >> 4;
                            let dr = sb & 0b00001111;

                            NCMacCommands::ForceRejoinReq { period, max_retries, rejoin_type, dr }
                        } else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x0F => {
                        if let Some(&payload) = bytes_iterator.next() {
                            let max_time_n  = (payload & 0b11110000) >> 4;
                            let max_count_n =  payload & 0b00001111;

                            NCMacCommands::RejoinParamSetupReq { max_time_n, max_count_n }
                        }
                        else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x10 => NCMacCommands::PingSlotInfoAns,
                    0x11 => {
                        let freq_b0 = bytes_iterator.next();
                        let freq_b1 = bytes_iterator.next();
                        let freq_b2 = bytes_iterator.next();
                        let data_rate = bytes_iterator.next();
                        if let (Some(&fb0), Some(&fb1), Some(&fb2), Some(&dr)) = (freq_b0, freq_b1, freq_b2, data_rate) {
                            let frequency = u32::from_le_bytes([fb0, fb1, fb2, 0]);
                            NCMacCommands::PingSlotChannelReq { frequency, data_rate: dr & 0b00001111 }
                        } else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x13 => {
                        let freq_b0 = bytes_iterator.next();
                        let freq_b1 = bytes_iterator.next();
                        let freq_b2 = bytes_iterator.next();
                        if let (Some(&fb0), Some(&fb1), Some(&fb2)) = (freq_b0, freq_b1, freq_b2) {
                            let frequency = u32::from_le_bytes([fb0, fb1, fb2, 0]);
                            NCMacCommands::BeaconFreqReq { frequency }
                        } else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }
                    },
                    0x20 => {
                        if let Some(&class) = bytes_iterator.next() {
                            NCMacCommands::DeviceModeConf(DeviceClass::from_byte(class))
                        }
                        else {
                            return Err(LoRaWANError::MalformedMACCommand)
                        }

                    }
                    code if (0x80..=0xff).contains(&b) => {
                        let payload: Vec<u8> = bytes_iterator.by_ref().copied().collect();
                        NCMacCommands::Proprietary(code, payload)
                    }
                    _ => { //invalid code
                        return Err(LoRaWANError::MalformedMACCommand)
                    }
                    
                };
                mac_commands_vec.push(new_command);
            }

            Ok(mac_commands_vec)
        }
    }
}

impl ToBytes for EDMacCommands {
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            EDMacCommands::ResetInd(version) => {
                vec![0x01, version & 0b00001111]
            },
            EDMacCommands::LinkCheckReq => {
                vec![0x02]
            },
            EDMacCommands::LinkADRAns { power_ack, data_rate_ack, channel_mask_ack } => {
                let mut status = 0;
                if *power_ack        { status |= 0b00000100 }
                if *data_rate_ack    { status |= 0b00000010 }
                if *channel_mask_ack { status |= 0b00000001 }
                vec![0x03, status]
            },
            EDMacCommands::DutyCycleAns => {
                vec![0x04]
            },
            EDMacCommands::RXParamSetupAns { rx1_dr_offset_ack,rx2_data_rate_ack, channel_ack } => {
                let mut status = 0;
                if *rx1_dr_offset_ack   { status |= 0b00000100 }
                if *rx2_data_rate_ack   { status |= 0b00000010 }
                if *channel_ack         { status |= 0b00000001 }
                vec![0x05, status]
            },
            EDMacCommands::DevStatusAns { battery, margin } => {
                vec![0x06, *battery, margin & 0b00111111]
            },
            EDMacCommands::NewChannelAns { data_range_ok, channel_frequency_ok } => {
                let mut status = 0;
                if *data_range_ok          { status |= 0b00000010 }
                if *channel_frequency_ok   { status |= 0b00000001 }
                vec![0x07, status]
            },
            EDMacCommands::RXTimingSetupAns => {
                vec![0x08]
            },
            EDMacCommands::TxParamSetupAns => {
                vec![0x09]
            },
            EDMacCommands::DlChannelAns { uplink_frequency_exists, channel_frequency_ok } => {
                let mut status = 0;
                if *uplink_frequency_exists  { status |= 0b00000010 }
                if *channel_frequency_ok     { status |= 0b00000001 }
                vec![0x0A, status]
            },
            EDMacCommands::RekeyInd(version) => {
                vec![0x0B, version & 0b00001111]
            },
            EDMacCommands::ADRParamSetupAns => {
                vec![0x0C]
            },
            EDMacCommands::DeviceTimeReq => {
                vec![0x0D]
            },
            EDMacCommands::RejoinParamSetupAns { time_ack } => {
                let mut ans = 0;
                if *time_ack { ans |= 0b00000001 }
                vec![0x0F, ans]
            },
            EDMacCommands::PingSlotInfoReq { periodicity } => {
                vec![0x10, periodicity & 0b00000011]
            },
            EDMacCommands::PingSlotChannelAns { data_rate_ok, channel_frequency_ok } => {
                let mut ans = 0;
                if *data_rate_ok { ans |= 0b00000010 }
                if *channel_frequency_ok { ans |= 0b00000001 }
                vec![0x11, ans]
            },
            EDMacCommands::BeaconFreqAns { beacon_freq_ok } => {
                let mut ans = 0;
                if *beacon_freq_ok { ans |= 0b00000001 }
                vec![0x13, ans]
            },
            EDMacCommands::DeviceModeInd(class) => {
                vec![0x20, class.to_byte()]
            },
            EDMacCommands::Proprietary(code, payload) => {
                let mut r = vec![0; payload.len() + 1];
                let slice = &mut r[1..];
                slice.copy_from_slice(payload);
                r[0] = *code;
                r
            },
        }
    }
}

impl ToBytes for NCMacCommands {
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            NCMacCommands::ResetConf(version) => {
                vec![0x01, version & 0b00001111]
            },
            NCMacCommands::LinkCheckAns { margin, gw_cnt } => {
                vec![0x02, *margin, *gw_cnt]
            },
            NCMacCommands::LinkADRReq { data_rate, tx_power, ch_mask, ch_mask_cntl, nb_trans } => {
                let data_rate_tx_power = ((data_rate & 0b00001111) << 4) | (tx_power & 0b00001111);
                let redundancy = ((ch_mask_cntl & 0b00000111) << 4) | (nb_trans & 0b00001111);
                let ch_mask_bytes:[u8; 2] = ch_mask.to_le_bytes();
                vec![0x03, data_rate_tx_power, ch_mask_bytes[0], ch_mask_bytes[1], redundancy]                
            },
            NCMacCommands::DutyCycleReq(b) => {
                vec![0x04, 0b00001111 & b]                
            },
            NCMacCommands::RXParamSetupReq { rx1_dr_offset, rx2_data_rate, freq } => {
                let mut dl_settings = 0;
                dl_settings |= (rx1_dr_offset & 0b00000111) << 4;
                dl_settings |= rx2_data_rate & 0b00001111;
                let frequency: [u8; 4] = freq.to_le_bytes();
                vec![0x05, dl_settings, frequency[0], frequency[1], frequency[2]]               
            },
            NCMacCommands::DevStatusReq => {
                vec![0x06]                
            },
            NCMacCommands::NewChannelReq { ch_index, freq, max_dr, min_dr } => {
                let data_range = ((max_dr & 0b00001111) << 4 ) | (min_dr & 0b00001111);
                let frequency: [u8; 4] = freq.to_le_bytes();
                vec![0x07, *ch_index, frequency[0], frequency[1], frequency[2], data_range]
            },
            NCMacCommands::RXTimingSetupReq(settings) => {
                vec![0x08, settings & 0b00001111]                
            },
            NCMacCommands::TxParamSetupReq { downlink_dwell_time, uplink_dwell_time, max_eirp }=> {
                let mut eirp_dwell_time = 0;
                if *downlink_dwell_time { eirp_dwell_time |= 0b00100000 }
                if *uplink_dwell_time   { eirp_dwell_time |= 0b00010000 }
                eirp_dwell_time |= max_eirp & 0b00001111;
                vec![0x09, eirp_dwell_time]                
            },
            NCMacCommands::DlChannelReq { ch_index, freq } => {
                let frequency: [u8; 4] = freq.to_le_bytes();
                vec![0x0A, *ch_index, frequency[0], frequency[1], frequency[2]]
            },
            NCMacCommands::RekeyConf(version) => {
                vec![0x0B, version & 0b00001111]                
            },
            NCMacCommands::ADRParamSetupReq { limit_exp, delay_exp }=> {
                let mut adr_params = 0;
                adr_params |= (limit_exp & 0b00001111) << 4;
                adr_params |= delay_exp  & 0b00001111;
                vec![0x0C, adr_params]                
            },
            NCMacCommands::DeviceTimeAns { epoch, second_fraction } => {
                let epoch_as_bytes: [u8; 4] = epoch.to_le_bytes();
                vec![0x0D, epoch_as_bytes[0], epoch_as_bytes[1], epoch_as_bytes[2], epoch_as_bytes[3], *second_fraction]
            },
            NCMacCommands::ForceRejoinReq { period, max_retries, rejoin_type, dr }=> {
                let mut first_byte = 0;
                let mut second_byte = 0;

                first_byte |= (period     & 0b00000111) << 3;
                first_byte |= max_retries & 0b00000111;
                
                second_byte |= (rejoin_type & 0b00000111) << 4;
                second_byte |= dr  & 0b00001111;

                vec![0x0E, first_byte, second_byte]                
            },
            NCMacCommands::RejoinParamSetupReq { max_time_n, max_count_n }=> {
                let mut payload = 0;
                
                payload |= (max_time_n  & 0b00001111) << 4;
                payload |= max_count_n & 0b00001111;

                vec![0x0F, payload]
            },
            NCMacCommands::PingSlotInfoAns => {
                vec![0x10]
            },
            NCMacCommands::PingSlotChannelReq { frequency, data_rate } => {
                let fb: [u8; 4] = frequency.to_le_bytes();
                vec![0x11, fb[0], fb[1], fb[2], data_rate & 0b00001111]
            },
            NCMacCommands::BeaconFreqReq { frequency } => {
                let fb: [u8; 4] = frequency.to_le_bytes();
                vec![0x13, fb[0], fb[1], fb[2]]
            },

            NCMacCommands::DeviceModeConf(class) => {
                vec![0x20, class.to_byte()]
            },
            
            NCMacCommands::Proprietary(code, payload) => {
                let mut r = vec![0; payload.len() + 1];
                let slice = &mut r[1..];
                slice.copy_from_slice(payload);
                r[0] = *code;
                r
            },
        }
    }
}