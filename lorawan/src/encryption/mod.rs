use openssl::symm::{Cipher, Crypter, Mode};
use crate::utils::{self, errors::LoRaWANError};
use self::key::Key;
pub mod key;


pub fn aes_128_encrypt_with_padding(key: &Key, plain_data: &mut Vec<u8>) -> Result<Vec<u8>, LoRaWANError> {
    if plain_data.len() % 16 != 0 {
        utils::pad_to_16(plain_data)
    }
    aes_128_encrypt(key, plain_data)    
}

pub fn aes_128_decrypt_with_padding(key: &Key, encrypted_data: &mut Vec<u8>) -> Result<Vec<u8>, LoRaWANError> {
    if encrypted_data.len() % 16 != 0 {
        utils::pad_to_16(encrypted_data)
    }
    aes_128_decrypt(key, encrypted_data)    
}


pub fn aes_128_encrypt(key: &Key, plain_data: &[u8]) -> Result<Vec<u8>, LoRaWANError> {
    if plain_data.len() % 16 != 0 {
        return Err(LoRaWANError::InvalidBufferLength);
    }
    let cipher = Cipher::aes_128_ecb();
    //let mut ciphred = openssl::symm::encrypt(cipher, &**key, Some(&[0; 16]), plain_data)?;
    //ciphred.truncate(cipher.block_size());
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, &**key, Some(&[0;16]))?;
    crypter.pad(false);
    
    let mut encrypted_data = vec![0; plain_data.len() + cipher.block_size()];
    let mut count = crypter.update(plain_data, &mut encrypted_data)?;
    count += crypter.finalize(&mut encrypted_data)?;
    encrypted_data.truncate(count);
    Ok(encrypted_data)
}

pub fn aes_128_decrypt(key: &Key, encrypted_data: &[u8]) -> Result<Vec<u8>, LoRaWANError> {
    let cipher = Cipher::aes_128_ecb();
    let mut crypter = Crypter::new(cipher, Mode::Decrypt, &**key, Some(&[0;16]))?;
    crypter.pad(false);
    
    let mut plain_data = vec![0; encrypted_data.len() + cipher.block_size()];
    let mut count = crypter.update(encrypted_data, &mut plain_data)?;
    count += crypter.finalize(&mut plain_data)?;
    plain_data.truncate(count);
    
    Ok(plain_data)
    //openssl::symm::decrypt(Cipher::aes_128_cbc(), &**key, None, encrypted_data)
}

pub fn aes_128_cmac(key: &Key, plain_data: &[u8]) -> Result<Vec<u8>, LoRaWANError> {
    let cmac_key = openssl::pkey::PKey::cmac(&Cipher::aes_128_cbc(), &**key)?;
    let mut signer = openssl::sign::Signer::new_without_digest(&cmac_key)?;
    signer.sign_oneshot_to_vec(plain_data).map_err(LoRaWANError::OpenSSLErrorStack)
}

pub fn extract_mic(key: &Key, plain_data: &[u8]) -> Result<[u8; 4], LoRaWANError> {
    let data = aes_128_cmac(key, plain_data)?;
    let data: [u8; 4] = [data[0], data[1], data[2], data[3]];
    Ok(data)
}