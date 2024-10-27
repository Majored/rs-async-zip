// Copyright (c) 2024 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::core::raw;
use crate::utils::read_u16;
use crate::utils::read_u32;
use crate::utils::write_u16;
use crate::utils::write_u32;

use futures_lite::io::AsyncWriteExt;

pub const SIGNATURE: u32 = 0x4034b50;

raw! {
    RawLocalFileHeader {
        version_needed_to_extract, u16, read_u16, write_u16,
        general_purpose_flags, u16, read_u16, write_u16,
        compression_method, u16, read_u16, write_u16,
        last_mod_file_time, u16, read_u16, write_u16,
        last_mod_file_date, u16, read_u16, write_u16,
        crc_32, u32, read_u32, write_u32,
        compressed_size, u32, read_u32, write_u32,
        uncompressed_size, u32, read_u32, write_u32,
        file_name_length, u16, read_u16, write_u16,
        extra_field_length, u16, read_u16, write_u16
    }
}

#[derive(Clone, Debug)]
pub struct LocalFileHeader {
    pub raw: RawLocalFileHeader,
    pub file_name: Vec<u8>,
    pub extra_field: Vec<u8>,
}

/// Reads a local file header from the given reader.
///
/// This function does so by:
/// - asserting the signature of the local file header
/// - reading the raw local file header
/// - reading the file name
/// - reading the extra field
#[tracing::instrument(skip(reader))]
pub async fn read(mut reader: impl AsyncRead + Unpin) -> Result<LocalFileHeader> {
    crate::utils::assert_signature(&mut reader, SIGNATURE).await?;

    let raw = raw_read(&mut reader).await?;
    let file_name = crate::utils::read_bytes(&mut reader, raw.file_name_length as usize).await?;
    let extra_field = crate::utils::read_bytes(&mut reader, raw.extra_field_length as usize).await?;

    Ok(LocalFileHeader {
        raw,
        file_name,
        extra_field,
    })
}

/// Writes a local file header to the given writer.
/// 
/// This function does so by:
/// - writing the signature of the local file header
/// - writing the raw local file header
/// - writing the file name
/// - writing the extra field
#[tracing::instrument(skip(writer))]
pub async fn write(mut writer: impl AsyncWrite + Unpin, header: &LocalFileHeader) -> Result<()> {
    crate::utils::write_u32(&mut writer, SIGNATURE).await?;

    raw_write(&mut writer, &header.raw).await?;
    writer.write_all(&header.file_name).await?;
    writer.write_all(&header.extra_field).await?;

    Ok(())
}