// Copyright (c) 2024 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::core::raw::eocdr::EndOfCentralDirectoryRecord;
use crate::core::raw::cdr::CentralDirectoryRecord;
use crate::error::Result;

use futures_lite::io::{AsyncBufRead, AsyncWrite};

/// Reads the central directory from the given reader.
///
/// This function does so by:
/// - reading the provided number of central directory records
#[tracing::instrument(skip(reader))]
pub async fn read(mut reader: impl AsyncBufRead + Unpin, num: u16) -> Result<Vec<CentralDirectoryRecord>> {
    let mut records = Vec::with_capacity(num as usize);

    for _ in 0..num {
        records.push(crate::core::raw::cdr::read(&mut reader).await?);
    }

    // We _should_ already have the EOCDR, so there's no point reading it again.

    Ok(records)
}

/// Writes a central directory record to the given writer.
///
/// This function does so by:
/// - writing the provided central directory records
/// - writing the provided end of central directory record
#[tracing::instrument(skip(writer))]
pub async fn write(
    mut writer: impl AsyncWrite + Unpin,
    records: &[CentralDirectoryRecord],
    ecodr: &EndOfCentralDirectoryRecord,
) -> Result<()> {
    for record in records {
        crate::core::raw::cdr::write(&mut writer, record).await?;
    }

    crate::core::raw::eocdr::write(&mut writer, ecodr).await?;

    Ok(())
}
