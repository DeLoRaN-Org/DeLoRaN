use std::{convert::TryInto, fmt::Display};

use serde::{Serialize, Deserialize};

use crate::{
    encryption::{aes_128_encrypt_with_padding, key::Key},
    utils::{errors::LoRaWANError, eui::EUI64, PrettyHexSlice},
};

#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkSessionContext {
    fnwk_s_int_key: Key,
    snwk_s_int_key: Key,
    nwk_s_enc_key: Key,

    home_net_id: [u8; 3],
    dev_addr: [u8; 4],
    f_cnt_up: u32,
    nf_cnt_dwn: u32,

    rj_count0: u16,
}

#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplicationSessionContext {
    app_s_key: Key, // -> //TODO questo va nell'application server in realtà
    //f_cnt_up: u32,
    af_cnt_dwn: u32,
}

#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct JoinSessionContext {
    js_int_key: Key,
    js_enc_key: Key,
    rj_count1: u16,
    join_nonce: u32,
}

#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionContext {
    application_context: ApplicationSessionContext,
    network_context: NetworkSessionContext,
}

impl NetworkSessionContext {
    pub fn derive(
        opt_neg: bool,
        nwk_key: &Key,
        join_nonce: &[u8; 3],
        join_eui: EUI64,
        dev_nonce: u32,
        dev_addr: &[u8; 4],
        net_id: &[u8; 3],
    ) -> Result<Self, LoRaWANError> {
        let dev_nonce_block: [u8; 2] = (dev_nonce as u16).to_be_bytes();

        if opt_neg {
            let eui = *join_eui;
            let mut block = vec![
                0x01,
                join_nonce[2],
                join_nonce[1],
                join_nonce[0],
                eui[7],
                eui[6],
                eui[5],
                eui[4],
                eui[3],
                eui[2],
                eui[1],
                eui[0],
                dev_nonce_block[0],
                dev_nonce_block[1],
                0,
                0,
            ];

            let fnwk_s_int_key: Key =
                aes_128_encrypt_with_padding(nwk_key, &mut block)?.try_into()?;

            block[0] = 0x03;
            let snwk_s_int_key: Key =
                aes_128_encrypt_with_padding(nwk_key, &mut block)?.try_into()?;

            block[0] = 0x04;
            let nwk_s_enc_key: Key =
                aes_128_encrypt_with_padding(nwk_key, &mut block)?.try_into()?;

            Ok(Self {
                fnwk_s_int_key,
                snwk_s_int_key,
                nwk_s_enc_key,
                dev_addr: *dev_addr,
                home_net_id: *net_id,
                f_cnt_up: 0,
                nf_cnt_dwn: 0,
                rj_count0: 0,
            })
        } else {
            let mut block = vec![
                0x01,
                join_nonce[2],
                join_nonce[1],
                join_nonce[0],
                net_id[2],
                net_id[1],
                net_id[0],
                dev_nonce_block[1],
                dev_nonce_block[0],
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ];

            let fnwk_s_int_key: Key =
                aes_128_encrypt_with_padding(nwk_key, &mut block)?.try_into()?;

            Ok(Self {
                fnwk_s_int_key,
                snwk_s_int_key: fnwk_s_int_key,
                nwk_s_enc_key: fnwk_s_int_key,
                dev_addr: *dev_addr,
                home_net_id: *net_id,
                f_cnt_up: 0,
                nf_cnt_dwn: 0,
                rj_count0: 0,
            })
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        fnwk_s_int_key: Key,
        snwk_s_int_key: Key,
        nwk_s_enc_key: Key,
        home_net_id: [u8; 3],
        dev_addr: [u8; 4],
        f_cnt_up: u32,
        nf_cnt_dwn: u32,
        rj_count0: u16,
    ) -> Self {
        Self {
            fnwk_s_int_key,
            snwk_s_int_key,
            nwk_s_enc_key,
            home_net_id,
            dev_addr,
            f_cnt_up,
            nf_cnt_dwn,
            rj_count0,
        }
    }

    /// Get the network session context's fnwk s int key.
    pub fn fnwk_s_int_key(&self) -> &Key {
        &self.fnwk_s_int_key
    }

    /// Get the network session context's snwk s int key.
    pub fn snwk_s_int_key(&self) -> &Key {
        &self.snwk_s_int_key
    }

    /// Get the network session context's nwk s enc key.
    pub fn nwk_s_enc_key(&self) -> &Key {
        &self.nwk_s_enc_key
    }

    /// Get the network session context's f cnt up.
    pub fn f_cnt_up(&self) -> u32 {
        self.f_cnt_up
    }
    
    pub fn update_f_cnt_up(&mut self, v: u32) {
        self.f_cnt_up = v
    }
    
    /// Get the network session context's f cnt up.
    pub fn f_cnt_up_autoinc(&mut self) -> u32 {
        self.f_cnt_up += 1;
        self.f_cnt_up
    }

    /// Get the network session context's nf cnt dwn.
    pub fn update_nf_cnt_dwn(&mut self, v: u32) {
        self.nf_cnt_dwn = v;
    }
    
    pub fn nf_cnt_dwn(&self) -> u32 {
        self.nf_cnt_dwn
    }

    pub fn nf_cnt_dwn_autoinc(&mut self) -> u32 {
        self.nf_cnt_dwn += 1;
        self.nf_cnt_dwn
    }

    pub fn update_rj_count0(&mut self, v: u16) {
        self.rj_count0 = v;
    }
    
    pub fn rj_count0(&self) -> u16 {
        self.rj_count0
    }

    pub fn rj_count0_autoinc(&mut self) -> u16 {
        self.rj_count0 += 1;
        self.rj_count0
    }

    pub fn dev_addr(&self) -> &[u8; 4] {
        &self.dev_addr
    }

    /// Get the network session context's home net id.
    pub fn home_net_id(&self) -> [u8; 3] {
        self.home_net_id
    }
}

impl ApplicationSessionContext {
    pub fn derive(
        opt_neg: bool,
        app_key: &Key,
        join_nonce: &[u8; 3],
        net_id: &[u8; 3],
        join_eui: EUI64,
        dev_nonce: u32,
    ) -> Result<Self, LoRaWANError> {
        let dev_nonce_block: [u8; 2] = (dev_nonce as u16).to_be_bytes();

        let mut block = if opt_neg {
            let eui = *join_eui;
            vec![
                0x02,
                join_nonce[2], join_nonce[1], join_nonce[0],
                eui[7], eui[6], eui[5], eui[4], eui[3], eui[2], eui[1], eui[0],
                dev_nonce_block[1], dev_nonce_block[0],
                0, 0,
            ]
        } else {
            vec![
                0x02,
                join_nonce[2], join_nonce[1], join_nonce[0],
                net_id[2], net_id[1], net_id[0],
                dev_nonce_block[1], dev_nonce_block[0],
                0,0,0,0,0,0,0,
            ]
        };

        let app_s_key: Key = aes_128_encrypt_with_padding(app_key, &mut block)?.try_into()?;

        Ok(Self {
            app_s_key,
            af_cnt_dwn: 0,
            //f_cnt_up: 0,
        })
    }

    pub fn new(app_s_key: Key, af_cnt_dwn: u32) -> Self {
        Self {
            app_s_key,
            //f_cnt_up,
            af_cnt_dwn,
        }
    }

    /// Get the application session context's app s key.
    pub fn app_s_key(&self) -> &Key {
        &self.app_s_key
    }

    /// Get the application session context's af cnt dwn.
    pub fn update_af_cnt_dwn(&mut self, v: u32) {
        self.af_cnt_dwn = v;
    }
    
    pub fn af_cnt_dwn(&self) -> u32 {
        self.af_cnt_dwn
    }

    pub fn af_cnt_dwn_autoinc(&mut self) -> u32 {
        self.af_cnt_dwn += 1;
        self.af_cnt_dwn
    }

    // Get the application session context's f cnt up.
    //pub fn f_cnt_up(&self) -> u32 {
    //    self.f_cnt_up
    //}

    //pub fn set_f_cnt_up(&mut self, v: u32) {
    //    self.f_cnt_up = v
    //}
}

impl JoinSessionContext {
    pub fn derive(nwk_key: &Key, dev_eui: &EUI64) -> Result<Self, LoRaWANError> {
        let mut buffer = Vec::from(**dev_eui);
        buffer.insert(0, 0x06);
        let js_int_key: Key = aes_128_encrypt_with_padding(nwk_key, &mut buffer)?.try_into()?;
        buffer[0] = 0x05;
        let js_enc_key: Key = aes_128_encrypt_with_padding(nwk_key, &mut buffer)?.try_into()?;

        Ok(Self {
            js_int_key,
            js_enc_key,
            rj_count1: 0,
            join_nonce: 0
        })
    }

    /// Get the join session context's js int key.
    pub fn js_int_key(&self) -> &Key {
        &self.js_int_key
    }

    /// Get the join session context's js enc key.
    pub fn js_enc_key(&self) -> &Key {
        &self.js_enc_key
    }

    pub fn rj_count1(&self) -> u16 {
        self.rj_count1
    }
    
    pub fn update_rj_count1(&mut self, v: u16) {
        self.rj_count1 = v;
    }

    pub fn rj_count1_autoinc(&mut self) -> u16 {
        self.rj_count1 += 1;
        self.rj_count1
    }

    pub fn update_join_nonce(&mut self, v: u32) {
        self.join_nonce = v;
    }
    
    pub fn join_nonce(&self) -> [u8; 3] {
        let ret = (self.join_nonce & 0x00FFFFFF).to_le_bytes(); //3 bytes
        [ret[0], ret[1], ret[2]]
    }

    pub fn join_nonce_autoinc(&mut self) -> [u8; 3] {
        self.join_nonce += 1;
        let ret = (self.join_nonce & 0x00FFFFFF).to_le_bytes();
        [ret[0], ret[1], ret[2]]
    }
}

impl SessionContext {

    #[allow(clippy::too_many_arguments)]
    pub fn derive(
        opt_neg: bool,
        nwk_key: &Key,
        app_key: &Key,
        join_nonce: &[u8; 3],
        join_eui: EUI64,
        dev_nonce: u32,
        dev_addr: &[u8; 4],
        net_id: &[u8; 3],
    ) -> Result<Self, LoRaWANError> {
        Ok(Self {
            application_context: ApplicationSessionContext::derive(
                opt_neg, app_key, join_nonce, net_id, join_eui, dev_nonce,
            )?,
            network_context: NetworkSessionContext::derive(
                opt_neg, nwk_key, join_nonce, join_eui, dev_nonce, dev_addr, net_id,
            )?,
        })
    }

    pub fn new(app_context: ApplicationSessionContext, nwk_context: NetworkSessionContext) -> Self {
        Self {
            application_context: app_context,
            network_context: nwk_context
        }
    }

    /// Get a reference to the session context's application.
    pub fn application_context(&self) -> &ApplicationSessionContext {
        &self.application_context
    }

    /// Get a reference to the session context's network.
    pub fn network_context(&self) -> &NetworkSessionContext {
        &self.network_context
    }

    /// Get a reference to the session context's application.
    pub fn application_context_mut(&mut self) -> &mut ApplicationSessionContext {
        &mut self.application_context
    }

    /// Get a reference to the session context's network.
    pub fn network_context_mut(&mut self) -> &mut NetworkSessionContext {
        &mut self.network_context
    }
}

impl Display for SessionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{
        {},
        {}  
    }}", self.application_context, self.network_context)
    }
}

impl Display for JoinSessionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"{{
        JsIntKey: {}
        JsEncKey: {}
        RJCount1: {}
    }}", self.js_int_key, self.js_enc_key, self.rj_count1)
    }
}

impl Display for ApplicationSessionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"ApplicationSessionContext: {{
            AppSKey: {}
            AFCntDown: {}
        }}", self.app_s_key/* , self.f_cnt_up*/, self.af_cnt_dwn)
    }
}

impl Display for NetworkSessionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"NetworkSessionContext: {{
            FNwkSIntKey: {}
            SNwkSIntKey: {}
            NwkSEncKey: {}
            HomeNetID: {}
            DevAddr: {}
            FCntUp: {}
            NFcntDown: {}
            RJCount0: {}
        }}", self.fnwk_s_int_key, self.snwk_s_int_key, self.nwk_s_enc_key, PrettyHexSlice(&self.home_net_id), PrettyHexSlice(&self.dev_addr), self.f_cnt_up, self.nf_cnt_dwn, self.rj_count0)
    }
}