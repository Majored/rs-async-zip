// Copyright (c) 2024 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::core::raw::{raw, raw_deref};
use crate::utils::{read_u16, write_u16};

use futures_lite::AsyncWriteExt;

raw! {
    RawExtensibleDataField {
        // Head ID - 2 bytes
        header_id, u16, read_u16, write_u16,
        // Data Size - 2 bytes
        data_size, u16, read_u16, write_u16
    }
}

#[derive(Clone, Debug)]
pub struct ExtensibleDataField {
    pub raw: RawExtensibleDataField,
    pub data: Vec<u8>,
}

raw_deref!(ExtensibleDataField, RawExtensibleDataField);

/// Reads an extensible data field from the provided reader.
///
/// This function does so by:
/// - reading the raw extensible data field
/// - reading the data of the extensible data field
#[tracing::instrument(skip(reader))]
pub async fn read(mut reader: impl AsyncBufRead + Unpin) -> Result<ExtensibleDataField> {
    let raw = raw_read(&mut reader).await?;
    let data = crate::utils::read_bytes(reader, raw.data_size as usize).await?;

    Ok(ExtensibleDataField { raw, data })
}

/// Writes an extensible data field to the provided writer.
///
/// This function does so by:
/// - writing the raw extensible data field
/// - writing the data of the extensible data field
#[tracing::instrument(skip(writer))]
pub async fn write(mut writer: impl AsyncWrite + Unpin, field: &ExtensibleDataField) -> Result<()> {
    raw_write(&mut writer, &field.raw).await?;
    writer.write_all(&field.data).await?;
    Ok(())
}
