// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::error::{Result, ZipError};
use tokio::io::{AsyncRead, AsyncReadExt};

/// Read and return a dynamic length string from a reader which impls AsyncRead.
pub async fn read_string<R: AsyncRead + Unpin>(reader: &mut R, length: usize) -> Result<String> {
    let mut buffer = String::with_capacity(length);
    reader.take(length as u64).read_to_string(&mut buffer).await?;

    Ok(buffer)
}

/// Read and return a dynamic length vector of bytes from a reader which impls AsyncRead.
pub async fn read_bytes<R: AsyncRead + Unpin>(reader: &mut R, length: usize) -> Result<Vec<u8>> {
    let mut buffer = Vec::with_capacity(length);
    reader.take(length as u64).read_to_end(&mut buffer).await?;

    Ok(buffer)
}

/// Assert that the next four-byte delimiter read by a reader which impls AsyncRead matches the expected delimiter.
pub(crate) async fn assert_delimiter<R: AsyncRead + Unpin>(reader: &mut R, expected: u32) -> Result<()> {
    match reader.read_u32_le().await? {
        actual if actual == expected => Ok(()),
        actual => Err(ZipError::UnexpectedHeaderError(actual, expected)),
    }
}
