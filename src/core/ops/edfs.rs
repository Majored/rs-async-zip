// Copyright (c) 2024 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::core::raw::edf::ExtensibleDataField;
use crate::error::Result;

use futures_lite::io::Cursor;

/// Reads all extensible data fields from the provided data.
///
/// This function does so by:
/// - creating a new cursor from the provided data
/// - reading all extensible data fields from the cursor
/// - returning once the data has been consumed
#[tracing::instrument()]
pub async fn read(data: &[u8]) -> Result<Vec<ExtensibleDataField>> {
    let mut cursor = Cursor::new(&data);
    let mut fields = Vec::new();

    while cursor.position() as usize != data.len() {
        fields.push(crate::core::raw::edf::read(&mut cursor).await?);
    }

    Ok(fields)
}
