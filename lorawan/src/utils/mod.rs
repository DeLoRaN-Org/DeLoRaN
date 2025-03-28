use std::{fmt::{Display, Write}, cmp::Ordering};

pub mod eui;
pub mod errors;
pub mod traits;

/// Pads the given buffer with zeros to make its length a multiple of 16.
///
/// # Arguments
///
/// * `buffer` - A mutable reference to a vector of `u8` elements.
pub fn pad_to_16(buffer: &mut Vec<u8>) {
    const CHUNK_SIZE: usize = 16;
    if buffer.len() % CHUNK_SIZE != 0 {
        let mut missing_pad = vec![0; CHUNK_SIZE - buffer.len() % CHUNK_SIZE];                
        buffer.append(&mut missing_pad);
    }
}

/// Checks if a received nonce is valid and if it has looped around.
///
/// # Arguments
///
/// * `received_nonce` - The received nonce value.
/// * `current_nonce` - The current nonce value.
///
/// # Returns
///
/// A tuple containing two boolean values:
/// * `nonce_valid` - Indicates if the received nonce is valid.
/// * `nonce_looped` - Indicates if the received nonce has looped around.
pub fn nonce_valid(received_nonce: u16, current_nonce: u16) -> (bool, bool) {
    match received_nonce.cmp(&current_nonce) {
        Ordering::Greater => (true, false),
        Ordering::Equal => (false, false),
        Ordering::Less => ((0xffff - current_nonce < 5) && received_nonce < 5, true),
    }
}

/// Increments the current nonce based on the received nonce and the looped flag.
///
/// # Arguments
///
/// * `received_nonce` - The received nonce value.
/// * `current_nonce` - The current nonce value.
/// * `nonce_looped` - Indicates if the received nonce has looped around.
///
/// # Returns
///
/// The incremented nonce value.
pub fn increment_nonce(received_nonce: u16, current_nonce: u32, nonce_looped: bool) -> u32 {
    let increment_higher_half_dev_nonce = if nonce_looped { 0x00010000 } else { 0 };
    received_nonce as u32 | ((current_nonce & 0xffff0000) + increment_higher_half_dev_nonce)
}

/// Wrapper struct for displaying a slice of bytes as a pretty hexadecimal string.
#[derive(PartialEq, Eq, Debug)]
pub struct PrettyHexSlice<'a>(pub &'a [u8]);

impl Display for PrettyHexSlice<'_> {
    /// Formats the slice of bytes as a pretty hexadecimal string.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the output to.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        for elem in self.0.iter() {
            write!(s, "{elem:02x}")?
        }
        write!(f, "{s}")
    }
}