// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use std::io::Cursor;

use binrw::{binrw, BinRead, BinWrite};
use futures_lite::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::error::Result;
use crate::spec::consts::{CDH_LENGTH, EOCDR_LENGTH, LFH_LENGTH};

#[binrw]
#[brw(little)]
#[brw(repr = u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signature {
    LFH = 0x04034b50,
    CDH = 0x02014b50,
    EOCDRH = 0x06054b50,
}

impl From<Signature> for u32 {
    fn from(sig: Signature) -> Self {
        sig as u32
    }
}

#[binrw]
#[brw(little)]
#[brw(repr = u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Compression {
    Stored = 0,
    Deflate = 8,
    Deflate64 = 9,
    Bz = 12,
    Lzma = 14,
    Zstd = 93,
    Xz = 95,
}

#[binrw]
#[brw(little)]
// Local file header
pub struct LFH {
    pub version: u16,
    pub flags: GPF,
    pub compression: Compression,
    pub mod_time: u16,
    pub mod_date: u16,
    pub crc: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub file_name_length: u16,
    pub extra_field_length: u16,
}

pub struct LF {
    pub lfh: LFH,
    pub file_name: Vec<u8>,
    pub extra_field: Vec<u8>,
}

#[binrw]
#[brw(little)]
#[derive(Clone)]
// Central directory record header
pub struct CDRH {
    pub v_made_by: u16,
    pub v_needed: u16,
    pub flags: GPF,
    pub compression: Compression,
    pub mod_time: u16,
    pub mod_date: u16,
    pub crc: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub file_name_length: u16,
    pub extra_field_length: u16,
    pub file_comment_length: u16,
    pub disk_start: u16,
    pub inter_attr: u16,
    pub exter_attr: u32,
    pub lh_offset: u32,
}

#[derive(Clone)]
pub struct CDR {
    pub cdrh: CDRH,
    pub file_name: Vec<u8>,
    pub extra_field: Vec<u8>,
    pub file_comment: Vec<u8>,
}

#[binrw]
#[brw(little)]
// End of central directory record header
pub struct EOCDRH {
    pub(crate) disk_num: u16,
    pub(crate) start_cent_dir_disk: u16,
    pub(crate) num_of_entries_disk: u16,
    pub(crate) num_of_entries: u16,
    pub(crate) size_cent_dir: u32,
    pub(crate) cent_dir_offset: u32,
    pub(crate) file_comm_length: u16,
}

pub struct EOCDR {
    pub eocdrh: EOCDRH,
    pub file_comment: Vec<u8>,
}

#[binrw]
#[brw(little)]
#[derive(Clone)]
// General purpose flags
pub struct GPF(u16);

impl GPF {
    pub fn data_descriptor(&self) -> bool {
        self.0 & 0x08 != 0
    }
}

pub(crate) trait HeaderSize {
    const SIZE: usize;
}

impl HeaderSize for Signature {
    const SIZE: usize = 4;
}

impl HeaderSize for LFH {
    const SIZE: usize = LFH_LENGTH;
}

impl HeaderSize for CDRH {
    const SIZE: usize = CDH_LENGTH;
}

impl HeaderSize for EOCDRH {
    const SIZE: usize = EOCDR_LENGTH;
}

/// Reads a fixed-size header from the given reader and returns the parsed struct.
pub(crate) async fn read<T, R>(reader: &mut R) -> Result<T>
where
    T: BinRead + HeaderSize,
    for<'a> T::Args<'a>: Default,
    R: AsyncRead + Unpin,
{
    let mut buffer = vec![0; T::SIZE];
    reader.read_exact(&mut buffer).await?;
    Ok(T::read_le(&mut Cursor::new(buffer))?)
}

/// Writes a fixed-size header to the given writer.
pub(crate) async fn write<T, W>(writer: &mut W, value: &T) -> Result<()>
where
    T: BinWrite,
    for<'a> T::Args<'a>: Default,
    W: AsyncWrite + Unpin,
{
    let mut buffer = Vec::new();
    value.write_le(&mut Cursor::new(&mut buffer))?;
    writer.write_all(&buffer).await?;
    Ok(())
}
