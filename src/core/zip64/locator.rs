// Copyright (c) 2024 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::core::{raw, raw_deref};
use crate::utils::{read_u32, read_u64, write_u32, write_u64};

pub const SIGNATURE: u32 = 0x07064b50;

raw! {
    RawZip64EndOfCentralDirectoryLocator {
        // number of the disk with the start of the zip64 end of central directory - 4 bytes
        disk_with_start_eocdr, u32, read_u32, write_u32,
        // relative offset of the zip64 end of central directory record - 8 bytes
        relative_offset_eocdr, u64, read_u64, write_u64,
        // total number of disks - 4 bytes
        total_disks, u32, read_u32, write_u32
    }
}

#[derive(Clone, Debug)]
pub struct Zip64EndOfCentralDirectoryLocator {
    pub raw: RawZip64EndOfCentralDirectoryLocator,
}

raw_deref!(Zip64EndOfCentralDirectoryLocator, RawZip64EndOfCentralDirectoryLocator);

/// Reads the ZIP64 end of central directory record from the given reader.
///
/// This function does so by:
/// - asserting the signature of the ZIP64 end of central directory record
/// - reading the raw ZIP64 end of central directory record
#[tracing::instrument(skip(reader))]
pub async fn read(mut reader: impl AsyncRead + Unpin) -> Result<Zip64EndOfCentralDirectoryLocator> {
    crate::utils::assert_signature(&mut reader, SIGNATURE).await?;
    let raw = raw_read(&mut reader).await?;
    Ok(Zip64EndOfCentralDirectoryLocator { raw })
}

/// Writes the ZIP64 end of central directory record to the given writer.
///
/// This function does so by:
/// - writing the signature of the ZIP64 end of central directory record
/// - writing the raw ZIP64 end of central directory record
#[tracing::instrument(skip(writer))]
pub async fn write(mut writer: impl AsyncWrite + Unpin, header: &Zip64EndOfCentralDirectoryLocator) -> Result<()> {
    crate::utils::write_u32(&mut writer, SIGNATURE).await?;
    raw_write(&mut writer, &header.raw).await?;
    Ok(())
}
