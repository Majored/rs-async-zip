// Copyright (c) 2024 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use futures_lite::io::Cursor;
use futures_lite::{AsyncBufRead, AsyncBufReadExt};

use crate::core::{raw, SIGNATURE_LENGTH};
use crate::utils::read_u32;
use crate::utils::write_u32;

pub const SIGNATURE: u32 = 0x8074b50;

raw! {
    RawDataDescriptor {
        crc_32, u32, read_u32, write_u32,
        compressed_size, u32, read_u32, write_u32,
        uncompressed_size, u32, read_u32, write_u32
    }
}

#[derive(Clone, Debug)]
pub struct DataDescriptor {
    pub raw: RawDataDescriptor,
}

/// Reads a data descriptor from the provided reader, ensuring to skip the signature if present.
///
/// This function does so by:
/// - getting the first four bytes from the reader
/// - consuming those bytes if they match the signature
/// - reading the raw data descriptor
#[tracing::instrument(skip(reader))]
pub async fn read(mut reader: impl AsyncBufRead + Unpin) -> Result<DataDescriptor> {
    let buffer = Cursor::new(reader.fill_buf().await?);
    let signature = read_u32(buffer).await?;

    if signature == SIGNATURE {
        reader.consume(SIGNATURE_LENGTH);
    }

    Ok(DataDescriptor { raw: raw_read(&mut reader).await? })
}

/// Writes a data descriptor to the provided writer.
///
/// This function does so by:
/// - writing the signature of the data descriptor
/// - writing the raw data descriptor
#[tracing::instrument(skip(writer))]
pub async fn write(mut writer: impl AsyncWrite + Unpin, header: &DataDescriptor) -> Result<()> {
    crate::utils::write_u32(&mut writer, SIGNATURE).await?;
    raw_write(&mut writer, &header.raw).await?;
    Ok(())
}
