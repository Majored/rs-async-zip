// Copyright (c) 2024 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::core::raw;
use crate::utils::read_u16;
use crate::utils::read_u32;
use crate::utils::write_u16;
use crate::utils::write_u32;

use futures_lite::io::AsyncWriteExt;

pub const SIGNATURE: u32 = 0x2014b50;

raw! {
    RawCentralDirectoryRecord {
        version_made_by, u16, read_u16, write_u16,
        version_needed_to_extract, u16, read_u16, write_u16,
        general_purpose_bit_flag, u16, read_u16, write_u16,
        compression_method, u16, read_u16, write_u16,
        last_mod_file_time, u16, read_u16, write_u16,
        last_mod_file_date, u16, read_u16, write_u16,
        crc_32, u32, read_u32, write_u32,
        compressed_size, u32, read_u32, write_u32,
        uncompressed_size, u32, read_u32, write_u32,
        file_name_length, u16, read_u16, write_u16,
        extra_field_length, u16, read_u16, write_u16,
        file_comment_length, u16, read_u16, write_u16,
        disk_number_start, u16, read_u16, write_u16,
        internal_file_attributes, u16, read_u16, write_u16,
        external_file_attributes, u32, read_u32, write_u32,
        relative_offset_of_local_header, u32, read_u32, write_u32
    }
}

#[derive(Clone, Debug)]
pub struct CentralDirectoryRecord {
    pub raw: RawCentralDirectoryRecord,
    pub file_name: Vec<u8>,
    pub extra_field: Vec<u8>,
    pub file_comment: Vec<u8>,
}

/// Reads a central directory record from the given reader.
///
/// This function does so by:
/// - asserting the signature of the central directory record
/// - reading the raw central directory record
/// - reading the file name
/// - reading the extra field
/// - reading the file comment
#[tracing::instrument(skip(reader))]
pub async fn read(mut reader: impl AsyncRead + Unpin) -> Result<CentralDirectoryRecord> {
    crate::utils::assert_signature(&mut reader, SIGNATURE).await?;

    let raw = raw_read(&mut reader).await?;
    let file_name = crate::utils::read_bytes(&mut reader, raw.file_name_length as usize).await?;
    let extra_field = crate::utils::read_bytes(&mut reader, raw.extra_field_length as usize).await?;
    let file_comment = crate::utils::read_bytes(&mut reader, raw.file_comment_length as usize).await?;

    Ok(CentralDirectoryRecord { raw, file_name, extra_field, file_comment })
}

/// Writes a central directory record to the given writer.
///
/// This function does so by:
/// - writing the signature of the central directory record
/// - writing the raw central directory record
/// - writing the file name
/// - writing the extra field
/// - writing the file comment
#[tracing::instrument(skip(writer))]
pub async fn write(mut writer: impl AsyncWrite + Unpin, header: &CentralDirectoryRecord) -> Result<()> {
    crate::utils::write_u32(&mut writer, SIGNATURE).await?;

    raw_write(&mut writer, &header.raw).await?;
    writer.write_all(&header.file_name).await?;
    writer.write_all(&header.extra_field).await?;
    writer.write_all(&header.file_comment).await?;

    Ok(())
}
