// Copyright (c) 2024 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! ZipCrypto encryption implementation.
//!
//! This module implements the traditional PKZIP encryption (also known as ZipCrypto).
//! Note: This is considered weak encryption by modern standards, but is kept for compatibility.

/// CRC32 lookup table for ZipCrypto
const CRC32_TABLE: [u32; 256] = generate_crc32_table();

/// Generate CRC32 lookup table at compile time
const fn generate_crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0u32;
    while i < 256 {
        let mut crc = i;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i as usize] = crc;
        i += 1;
    }
    table
}

/// Update CRC32 with a single byte (ZipCrypto style)
fn crc32_update(crc: u32, byte: u8) -> u32 {
    (crc >> 8) ^ CRC32_TABLE[((crc as u8) ^ byte) as usize]
}

/// The size of the encryption header in bytes
pub const ENCRYPTION_HEADER_SIZE: usize = 12;

/// ZipCrypto cipher state
#[derive(Clone)]
pub struct ZipCrypto {
    keys: [u32; 3],
}

impl ZipCrypto {
    /// Initialize the cipher with a password
    pub fn new(password: &[u8]) -> Self {
        let mut cipher = Self { keys: [0x12345678, 0x23456789, 0x34567890] };
        for &byte in password {
            cipher.update_keys(byte);
        }
        cipher
    }

    /// Initialize with password and verify byte (for decryption)
    #[allow(dead_code)]
    pub fn new_with_verify(password: &[u8], verify_byte: u8) -> Self {
        let cipher = Self::new(password);
        // The verify byte is used to check if the password is correct
        // This is handled during decryption
        let _ = verify_byte;
        cipher
    }

    /// Update the internal key state with a byte
    fn update_keys(&mut self, byte: u8) {
        // Key0 is updated with CRC32
        self.keys[0] = crc32_update(self.keys[0], byte);
        // Key1 is updated with a modular multiplication
        self.keys[1] = self.keys[1].wrapping_add(self.keys[0] & 0xFF).wrapping_mul(134775813).wrapping_add(1);
        // Key2 is updated with CRC32
        self.keys[2] = crc32_update(self.keys[2], (self.keys[1] >> 24) as u8);
    }

    /// Decrypt a byte
    #[allow(dead_code)]
    pub fn decrypt_byte(&mut self, cipher_byte: u8) -> u8 {
        let plain = cipher_byte ^ self.decrypt_key();
        self.update_keys(plain);
        plain
    }

    /// Encrypt a byte
    pub fn encrypt_byte(&mut self, plain_byte: u8) -> u8 {
        let cipher = plain_byte ^ self.decrypt_key();
        self.update_keys(plain_byte);
        cipher
    }

    /// Get the current decryption key (used for encryption too)
    fn decrypt_key(&self) -> u8 {
        let temp = self.keys[2] | 2;
        ((temp.wrapping_mul(temp ^ 1)) >> 8) as u8
    }

    /// Generate encryption header for encryption
    pub fn encrypt_header(&mut self, verify_byte: u8) -> Vec<u8> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut header = Vec::with_capacity(ENCRYPTION_HEADER_SIZE);

        // Generate 11 random bytes
        for _ in 0..11 {
            let random_byte: u8 = rng.gen();
            header.push(random_byte ^ self.decrypt_key());
            self.update_keys(random_byte);
        }

        // The 12th byte is a verify byte (typically high byte of CRC or file time)
        header.push(verify_byte ^ self.decrypt_key());
        self.update_keys(verify_byte);

        header
    }

    /// Encrypt a slice of data in place
    pub fn encrypt_data(&mut self, data: &mut [u8]) {
        for byte in data.iter_mut() {
            *byte = self.encrypt_byte(*byte);
        }
    }

    /// Encrypt a slice of data, returning new encrypted data
    #[allow(dead_code)]
    pub fn encrypt_data_owned(&mut self, data: &[u8]) -> Vec<u8> {
        let mut result = data.to_vec();
        self.encrypt_data(&mut result);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let password = b"test";
        let data = b"Hello, World!";

        // Encrypt
        let mut encryptor = ZipCrypto::new(password);
        let header = encryptor.encrypt_header(0);
        let mut encrypted = data.to_vec();
        encryptor.encrypt_data(&mut encrypted);

        // Decrypt - reinitialize with same password and process the same header bytes
        let mut decryptor = ZipCrypto::new(password);
        let mut decrypted = Vec::with_capacity(data.len());

        // Process header bytes to get to the same state
        for &byte in &header {
            decryptor.decrypt_byte(byte);
        }

        // Decrypt data
        for &byte in &encrypted {
            decrypted.push(decryptor.decrypt_byte(byte));
        }

        assert_eq!(decrypted, data);
    }
}
