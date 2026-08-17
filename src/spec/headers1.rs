// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use std::io::Cursor;

use binrw::{binrw, BinRead, BinWrite};
use futures_lite::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
#[cfg(feature = "tracing")]
use tracing::{instrument, trace};

use crate::error::Result;

/// A trait for types that have a fixed size in bytes, such as headers.
/// 
/// We cannot use `std::mem::size_of::<T>()` because the types are not packed.
pub(crate) trait HeaderSize {
    const SIZE: usize;
}

#[binrw]
#[brw(little)]
#[brw(repr = u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signature {
    LFH = 0x04034b50,
    CDH = 0x02014b50,
    EOCDRH = 0x06054b50,
    EOCDR64H = 0x06064b50,
    EOCDL64H = 0x07064b50,
    DD = 0x08074b50,
}

impl From<Signature> for u32 {
    fn from(sig: Signature) -> Self {
        sig as u32
    }
}

impl HeaderSize for Signature {
    const SIZE: usize = 4;
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
#[brw(repr = u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExtraFieldHeaderId {
    EI64,
    IZUC,
    IZUP,
    Other(u16),
}

impl ExtraFieldHeaderId {
    const KNOWN: &'static [(u16, ExtraFieldHeaderId)] = &[
        (0x0001, Self::EI64),
        (0x6375, Self::IZUC),
        (0x7075, Self::IZUP),
    ];
}

impl From<u16> for ExtraFieldHeaderId {
    fn from(value: u16) -> Self {
        match Self::KNOWN.iter().find(|(tag, _)| *tag == value) {
            Some((_, id)) => *id,
            None => Self::Other(value),
        }
    }
}

impl From<&ExtraFieldHeaderId> for u16 {
    fn from(id: &ExtraFieldHeaderId) -> Self {
        match id {
            ExtraFieldHeaderId::Other(v) => *v,
            known => ExtraFieldHeaderId::KNOWN.iter().find(|(_, id)| id == known).map(|(tag, _)| *tag).unwrap(),
        }
    }
}

#[binrw]
#[brw(little)]
#[derive(Debug)]
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

impl HeaderSize for LFH {
    const SIZE: usize = 26;
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug)]
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

impl HeaderSize for CDRH {
    const SIZE: usize = 42;
}

#[binrw]
#[brw(little)]
#[derive(Debug)]
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

impl HeaderSize for EOCDRH {
    const SIZE: usize = 18;
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug)]
// General purpose flags
pub struct GPF(u16);

impl GPF {
    pub fn data_descriptor(&self) -> bool {
        self.0 & 0x08 != 0
    }

    pub fn language_encoding_flag(&self) -> bool {
        self.0 & 0x800 != 0
    }
}

#[binrw]
#[brw(little)]
/// ZIP64 end of central directory record header
pub struct EOCDR64H {
    pub size_of_zip64_end_of_cd_record: u64,
    pub version_made_by: u16,
    pub version_needed_to_extract: u16,
    pub disk_number: u32,
    pub disk_number_start_of_cd: u32,
    pub num_entries_in_directory_on_disk: u64,
    pub num_entries_in_directory: u64,
    pub directory_size: u64,
    pub offset_of_start_of_directory: u64,
}

impl HeaderSize for EOCDR64H {
    const SIZE: usize = 56;
}

#[binrw]
#[brw(little)]
/// ZIP64 end of central directory locator header
pub struct EOCDL64H {
    pub number_of_disk_with_start_of_zip64_end_of_central_directory: u32,
    pub relative_offset: u64,
    pub total_number_of_disks: u32,
}

impl HeaderSize for EOCDL64H {
    const SIZE: usize = 16;
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug)]
// Extra field header - not part of the ZIP structure itself, but the spec still describes it as a 'header'.
pub struct EFH {
    pub tag: ExtraFieldHeaderId,
    pub data_size: u16,
}

impl HeaderSize for EFH {
    const SIZE: usize = 4;
}

/// Reads a fixed-size header from the given reader and returns the parsed struct.
#[cfg_attr(feature = "tracing", instrument(skip(reader), level = "trace"))]
pub(crate) async fn read<T, R>(reader: &mut R) -> Result<T>
where
    T: BinRead + HeaderSize + std::fmt::Debug,
    for<'a> T::Args<'a>: Default,
    R: AsyncRead + Unpin,
{
    #[cfg(feature = "tracing")]
    trace!("reading header of size {:02X?}", T::SIZE);

    let mut buffer = vec![0; T::SIZE];
    reader.read_exact(&mut buffer).await?;
    #[cfg(feature = "tracing")]
    trace!("read buffer: {:02X?}", buffer);

    let header = T::read_le(&mut Cursor::new(buffer))?;
    #[cfg(feature = "tracing")]
    trace!("parsed header: {:?}", header);

    Ok(header)
}

/// Reads a variable-length record from the given reader and returns the parsed construct.
///
/// Every such record begins with a fixed-size header `H` which states the length of the
/// variable data following it. We read that header to learn how many more bytes belong to
/// the record, pull them, then hand the whole record to binrw as `T`. `H` is parsed twice
/// (once here, once as `T`'s first field) which is a few dozen bytes of work, and buys us
/// a single declarative definition of the record in `spec::constructs`.
///
/// `tail_len` is fallible so that callers can apply their configured limits to the lengths the
/// header declares, before anything is allocated or read on their behalf.
#[cfg_attr(feature = "tracing", instrument(skip(reader, tail_len), level = "trace"))]
pub(crate) async fn read_record<H, T, R>(reader: &mut R, tail_len: impl FnOnce(&H) -> Result<usize>) -> Result<T>
where
    H: BinRead + HeaderSize + std::fmt::Debug,
    for<'a> H::Args<'a>: Default,
    T: BinRead + std::fmt::Debug,
    for<'a> T::Args<'a>: Default,
    R: AsyncRead + Unpin,
{
    let mut buffer = vec![0; H::SIZE];
    reader.read_exact(&mut buffer).await?;

    let header = H::read_le(&mut Cursor::new(&buffer))?;
    #[cfg(feature = "tracing")]
    trace!("parsed header: {:?}", header);

    let offset = buffer.len();
    buffer.resize(offset + tail_len(&header)?, 0);
    reader.read_exact(&mut buffer[offset..]).await?;
    #[cfg(feature = "tracing")]
    trace!("read record buffer: {:02X?}", buffer);

    let record = T::read_le(&mut Cursor::new(buffer))?;
    #[cfg(feature = "tracing")]
    trace!("parsed record: {:?}", record);

    Ok(record)
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
