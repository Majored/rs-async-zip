use crate::array_push;
use std::convert::TryInto;

pub struct LocalFileHeader {
    pub version: u16,
    pub flags: GeneralPurposeFlag,
    pub compression: u16,
    pub mod_time: u16,
    pub mod_date: u16,
    pub crc: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub file_name_length: u16,
    pub extra_field_length: u16,
}

impl LocalFileHeader {
    pub fn to_slice(&self) -> [u8; 26] {
        let mut array = [0; 26];
        let mut cursor = 0;

        array_push!(array, cursor, self.version.to_le_bytes());
        array_push!(array, cursor, self.flags.to_slice());
        array_push!(array, cursor, self.compression.to_le_bytes());
        array_push!(array, cursor, self.mod_time.to_le_bytes());
        array_push!(array, cursor, self.mod_date.to_le_bytes());
        array_push!(array, cursor, self.crc.to_le_bytes());
        array_push!(array, cursor, self.compressed_size.to_le_bytes());
        array_push!(array, cursor, self.uncompressed_size.to_le_bytes());
        array_push!(array, cursor, self.file_name_length.to_le_bytes());
        array_push!(array, cursor, self.extra_field_length.to_le_bytes());

        array
    }
}

impl From<[u8; 26]> for LocalFileHeader {
    fn from(value: [u8; 26]) -> LocalFileHeader {
        LocalFileHeader {
            version: u16::from_le_bytes(value[0..2].try_into().unwrap()),
            flags: GeneralPurposeFlag::from(u16::from_le_bytes(value[2..4].try_into().unwrap())),
            compression: u16::from_le_bytes(value[4..6].try_into().unwrap()),
            mod_time: u16::from_le_bytes(value[6..8].try_into().unwrap()),
            mod_date: u16::from_le_bytes(value[8..10].try_into().unwrap()),
            crc: u32::from_le_bytes(value[10..14].try_into().unwrap()),
            compressed_size: u32::from_le_bytes(value[14..18].try_into().unwrap()),
            uncompressed_size: u32::from_le_bytes(value[18..22].try_into().unwrap()),
            file_name_length: u16::from_le_bytes(value[22..24].try_into().unwrap()),
            extra_field_length: u16::from_le_bytes(value[24..26].try_into().unwrap()),
        }
    }
}

pub struct GeneralPurposeFlag {
    pub encrypted: bool,
    pub data_descriptor: bool,
}

impl GeneralPurposeFlag {
    pub fn to_slice(&self) -> [u8; 2] {
        let encrypted: u16 = match self.encrypted {
            false => 0x0,
            true => 0b1 << 14,
        };
        let data_descriptor: u16 = match self.data_descriptor {
            false => 0x0,
            true => 0b1 << 12,
        };

        (encrypted | data_descriptor).to_le_bytes()
    }
}

impl From<u16> for GeneralPurposeFlag {
    fn from(value: u16) -> GeneralPurposeFlag {
        let encrypted = match (value & 0x8000) >> 14 {
            0 => false,
            _ => true,
        };
        let data_descriptor = match (value & 0x1000) >> 12 {
            0 => false,
            _ => true,
        };

        GeneralPurposeFlag {
            encrypted,
            data_descriptor,
        }
    }
}

/// Replace elements of an array at a given cursor index for use with a zero-initialised array.
macro_rules! array_push {
    ($arr:ident, $cursor:ident, $value:expr) => {{
        for entry in $value {
            $arr[$cursor] = entry;
            $cursor += 1;
        }
    }};
}
