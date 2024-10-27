// Copyright (c) 2024 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::core::{raw, raw_deref};
use crate::error::ZipError;
use crate::utils::{read_u16, read_u32, write_u16, write_u32};

use futures_lite::io::AsyncWriteExt;

pub const SIGNATURE: u32 = 0x4034b50;

raw! {
    RawLocalFileHeader {
        // version needed to extract - 2 bytes
        version_needed, u16, read_u16, write_u16,
        // general purpose bit flag - 2 bytes
        flags, u16, read_u16, write_u16,
        // compression method - 2 bytes
        compression, u16, read_u16, write_u16,
        // last mod file time - 2 bytes
        last_mod_time, u16, read_u16, write_u16,
        // last mod file date - 2 bytes
        last_mod_date, u16, read_u16, write_u16,
        // crc-32 - 4 bytes
        crc, u32, read_u32, write_u32,
        // compressed_size - 4 bytes
        compressed_size, u32, read_u32, write_u32,
        // uncompressed_size - 4 bytes
        uncompressed_size, u32, read_u32, write_u32,
        // file name length - 2 bytes
        file_name_length, u16, read_u16, write_u16,
        // extra field length - 2 bytes
        extra_field_length, u16, read_u16, write_u16
    }
}

#[derive(Clone, Debug)]
pub struct LocalFileHeader {
    pub raw: RawLocalFileHeader,
    pub file_name: Vec<u8>,
    pub extra_field: Vec<u8>,
}

raw_deref!(LocalFileHeader, RawLocalFileHeader);

/// Reads a local file header from the given reader.
///
/// This function does so by:
/// - asserting the signature of the local file header
/// - reading the raw local file header
/// - reading the file name
/// - reading the extra field
#[tracing::instrument(skip(reader))]
pub async fn read(mut reader: impl AsyncBufRead + Unpin) -> Result<LocalFileHeader> {
    crate::utils::assert_signature(&mut reader, SIGNATURE).await?;

    let raw = raw_read(&mut reader).await?;
    let file_name = crate::utils::read_bytes(&mut reader, raw.file_name_length as usize).await?;
    let extra_field = crate::utils::read_bytes(&mut reader, raw.extra_field_length as usize).await?;

    Ok(LocalFileHeader { raw, file_name, extra_field })
}

/// Reads a local file header from the given reader in a streaming fashion.
///
/// This function does so by:
/// - matching against the next signature without consuming
/// - reading the local file header if we got the expected signature
/// - returning None if the signature is a CDR (meaning no more local file headers)
/// - returning an error if the signature is unexpected
#[tracing::instrument(skip(reader))]
pub async fn read_streaming(mut reader: impl AsyncBufRead + Unpin) -> Result<Option<LocalFileHeader>> {
    match crate::utils::check_signature(&mut reader, SIGNATURE).await? {
        SIGNATURE => Ok(Some(read(&mut reader).await?)),
        crate::core::cdr::SIGNATURE => return Ok(None),
        actual => return Err(ZipError::UnexpectedHeaderError(actual, SIGNATURE)),
    }
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
