// Copyright (c) 2024 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::core::{raw, raw_deref};
use crate::utils::{read_u16, read_u32, read_u64, write_u16, write_u32, write_u64};

pub const SIGNATURE: u32 = 0x06064b50;

raw! {
    RawZip64EndOfCentralDirectoryRecord {
        // size of zip64 end of central directory record - 8 bytes
        size_of_record, u64, read_u64, write_u64,
        // version made by - 2 bytes
        version_made_by, u16, read_u16, write_u16,
        // version needed to extract - 2 bytes
        version_needed, u16, read_u16, write_u16,
        // number of this disk - 4 bytes
        number_of_this_disk, u32, read_u32, write_u32,
        // number of the disk with the start of the central directory - 4 bytes
        disk_with_start_of_cd, u32, read_u32, write_u32,
        // total number of entries in the central directory on this disk - 8 bytes
        total_entries_in_cd_on_this_disk, u64, read_u64, write_u64,
        // total number of entries in the central directory - 8 bytes
        total_entries_in_cd, u64, read_u64, write_u64,
        // size of the central directory - 8 bytes
        size_of_cd, u64, read_u64, write_u64,
        // offset of start of central directory with respect to the starting disk number - 8 bytes
        offset_start_of_cd, u64, read_u64, write_u64
    }
}

#[derive(Clone, Debug)]
pub struct Zip64EndOfCentralDirectoryRecord {
    pub raw: RawZip64EndOfCentralDirectoryRecord,
}

raw_deref!(Zip64EndOfCentralDirectoryRecord, RawZip64EndOfCentralDirectoryRecord);

/// Reads the ZIP64 end of central directory record from the given reader.
///
/// This function does so by:
/// - asserting the signature of the ZIP64 end of central directory record
/// - reading the raw ZIP64 end of central directory record
#[tracing::instrument(skip(reader))]
pub async fn read(mut reader: impl AsyncRead + Unpin) -> Result<Zip64EndOfCentralDirectoryRecord> {
    crate::utils::assert_signature(&mut reader, SIGNATURE).await?;
    let raw = raw_read(&mut reader).await?;
    Ok(Zip64EndOfCentralDirectoryRecord { raw })
}

/// Writes the ZIP64 end of central directory record to the given writer.
///
/// This function does so by:
/// - writing the signature of the ZIP64 end of central directory record
/// - writing the raw ZIP64 end of central directory record
#[tracing::instrument(skip(writer))]
pub async fn write(mut writer: impl AsyncWrite + Unpin, header: &Zip64EndOfCentralDirectoryRecord) -> Result<()> {
    crate::utils::write_u32(&mut writer, SIGNATURE).await?;
    raw_write(&mut writer, &header.raw).await?;
    Ok(())
}
