// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::spec::header::{
    LocalFileHeader,
    GeneralPurposeFlag,
    CentralDirectoryHeader,
    EndOfCentralDirectoryHeader
};
use crate::error::Result;

use tokio::io::{AsyncRead, AsyncReadExt};

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

impl CentralDirectoryHeader {
    pub fn to_slice(&self) -> [u8; 42] {
        let mut array = [0; 42];
        let mut cursor = 0;

        array_push!(array, cursor, self.v_made_by.to_le_bytes());
        array_push!(array, cursor, self.v_needed.to_le_bytes());
        array_push!(array, cursor, self.flags.to_slice());
        array_push!(array, cursor, self.compression.to_le_bytes());
        array_push!(array, cursor, self.mod_time.to_le_bytes());
        array_push!(array, cursor, self.mod_date.to_le_bytes());
        array_push!(array, cursor, self.crc.to_le_bytes());
        array_push!(array, cursor, self.compressed_size.to_le_bytes());
        array_push!(array, cursor, self.uncompressed_size.to_le_bytes());
        array_push!(array, cursor, self.file_name_length.to_le_bytes());
        array_push!(array, cursor, self.extra_field_length.to_le_bytes());
        array_push!(array, cursor, self.file_comment_length.to_le_bytes());
        array_push!(array, cursor, self.disk_start.to_le_bytes());
        array_push!(array, cursor, self.inter_attr.to_le_bytes());
        array_push!(array, cursor, self.exter_attr.to_le_bytes());
        array_push!(array, cursor, self.lh_offset.to_le_bytes());

        array
    }
}

impl EndOfCentralDirectoryHeader {
    pub fn to_slice(&self) -> [u8; 18] {
        let mut array = [0; 18];
        let mut cursor = 0;

        array_push!(array, cursor, self.disk_num.to_le_bytes());
        array_push!(array, cursor, self.start_cent_dir_disk.to_le_bytes());
        array_push!(array, cursor, self.num_of_entries_disk.to_le_bytes());
        array_push!(array, cursor, self.num_of_entries.to_le_bytes());
        array_push!(array, cursor, self.size_cent_dir.to_le_bytes());
        array_push!(array, cursor, self.cent_dir_offset.to_le_bytes());
        array_push!(array, cursor, self.file_comm_length.to_le_bytes());

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

impl From<u16> for GeneralPurposeFlag {
    fn from(value: u16) -> GeneralPurposeFlag {
        let encrypted = match value & 0x1 {
            0 => false,
            _ => true,
        };
        let data_descriptor = match (value & 0x8) >> 3 {
            0 => false,
            _ => true,
        };

        GeneralPurposeFlag { encrypted, data_descriptor }
    }
}

impl From<[u8; 42]> for CentralDirectoryHeader {
    fn from(value: [u8; 42]) -> CentralDirectoryHeader {
        CentralDirectoryHeader {
            v_made_by: u16::from_le_bytes(value[0..2].try_into().unwrap()),
            v_needed: u16::from_le_bytes(value[2..4].try_into().unwrap()),
            flags: GeneralPurposeFlag::from(u16::from_le_bytes(value[4..6].try_into().unwrap())),
            compression: u16::from_le_bytes(value[6..8].try_into().unwrap()),
            mod_time: u16::from_le_bytes(value[8..10].try_into().unwrap()),
            mod_date: u16::from_le_bytes(value[10..12].try_into().unwrap()),
            crc: u32::from_le_bytes(value[12..16].try_into().unwrap()),
            compressed_size: u32::from_le_bytes(value[16..20].try_into().unwrap()),
            uncompressed_size: u32::from_le_bytes(value[20..24].try_into().unwrap()),
            file_name_length: u16::from_le_bytes(value[24..26].try_into().unwrap()),
            extra_field_length: u16::from_le_bytes(value[26..28].try_into().unwrap()),
            file_comment_length: u16::from_le_bytes(value[28..30].try_into().unwrap()),
            disk_start: u16::from_le_bytes(value[30..32].try_into().unwrap()),
            inter_attr: u16::from_le_bytes(value[32..34].try_into().unwrap()),
            exter_attr: u32::from_le_bytes(value[34..38].try_into().unwrap()),
            lh_offset: u32::from_le_bytes(value[38..42].try_into().unwrap()),
        }
    }
}

impl From<[u8; 18]> for EndOfCentralDirectoryHeader {
    fn from(value: [u8; 18]) -> EndOfCentralDirectoryHeader {
        EndOfCentralDirectoryHeader {
            disk_num: u16::from_le_bytes(value[0..2].try_into().unwrap()),
            start_cent_dir_disk: u16::from_le_bytes(value[2..4].try_into().unwrap()),
            num_of_entries_disk: u16::from_le_bytes(value[4..6].try_into().unwrap()),
            num_of_entries: u16::from_le_bytes(value[6..8].try_into().unwrap()),
            size_cent_dir: u32::from_le_bytes(value[8..12].try_into().unwrap()),
            cent_dir_offset: u32::from_le_bytes(value[12..16].try_into().unwrap()),
            file_comm_length: u16::from_le_bytes(value[16..18].try_into().unwrap()),
        }
    }
}

impl LocalFileHeader {
    pub async fn from_reader<R: AsyncRead + Unpin>(reader: &mut R) -> Result<LocalFileHeader> {
        let mut buffer: [u8; 26] = [0; 26];
        reader.read(&mut buffer).await?;
        Ok(LocalFileHeader::from(buffer))
    }
}

impl EndOfCentralDirectoryHeader {
    pub async fn from_reader<R: AsyncRead + Unpin>(reader: &mut R) -> Result<EndOfCentralDirectoryHeader> {
        let mut buffer: [u8; 18] = [0; 18];
        reader.read(&mut buffer).await?;
        Ok(EndOfCentralDirectoryHeader::from(buffer))
    }
}

impl CentralDirectoryHeader {
    pub async fn from_reader<R: AsyncRead + Unpin>(reader: &mut R) -> Result<CentralDirectoryHeader> {
        let mut buffer: [u8; 42] = [0; 42];
        reader.read(&mut buffer).await?;
        Ok(CentralDirectoryHeader::from(buffer))
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

pub(crate) use array_push;
