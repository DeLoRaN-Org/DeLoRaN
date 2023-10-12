use std::{fmt::{Display, Write}, cmp::Ordering};

pub mod eui;
pub mod errors;
pub mod traits;

pub fn pad_to_16(buffer: &mut Vec<u8>) {
    const CHUNK_SIZE: usize = 16;
    if buffer.len() % CHUNK_SIZE != 0 {
        let mut missing_pad = vec![0; CHUNK_SIZE - buffer.len() % CHUNK_SIZE];                
        buffer.append(&mut missing_pad);
    }
}

pub fn nonce_valid(received_nonce: u16, current_nonce: u16) -> (bool, bool) { // return values -> (nonce_valid, nonce_looped)
    match received_nonce.cmp(&current_nonce) {
        Ordering::Greater => (true, false),
        Ordering::Equal => (false, false),
        Ordering::Less => ((0xffff - current_nonce < 5) && received_nonce < 5, true),
    }
}

pub fn increment_nonce(received_nonce: u16, current_nonce: u32, nonce_looped: bool) -> u32 {
    let increment_higher_half_dev_nonce = if nonce_looped { 0x00010000 } else { 0 };
    received_nonce as u32 | ((current_nonce & 0xffff0000) + increment_higher_half_dev_nonce)
}

#[derive(PartialEq, Eq, Debug)]
pub struct PrettyHexSlice<'a>(pub &'a [u8]);

impl <'a> Display for PrettyHexSlice<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        for elem in self.0.iter() {
            write!(s, "{elem:02x}")?
        }
        write!(f, "{s}")
    }
}