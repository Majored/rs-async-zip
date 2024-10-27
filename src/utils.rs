// Copyright (c) 2023 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::error::{Result, ZipError};
use futures_lite::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

// Assert that the next four-byte signature read by a reader which impls AsyncRead matches the expected signature.
#[tracing::instrument(skip(reader))]
pub(crate) async fn assert_signature(reader: impl AsyncRead + Unpin, expected: u32) -> Result<()> {
    let actual = read_u32(reader).await?;

    if actual != expected {
        return Err(ZipError::UnexpectedHeaderError(actual, expected));
    }

    Ok(())
}

#[tracing::instrument(skip(reader))]
pub(crate) async fn read_u16(mut reader: impl AsyncRead + Unpin) -> Result<u16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf).await.unwrap();
    Ok(u16::from_le_bytes(buf))
}

#[tracing::instrument(skip(reader))]
pub(crate) async fn read_u32(mut reader: impl AsyncRead + Unpin) -> Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf).await.unwrap();
    Ok(u32::from_le_bytes(buf))
}

#[tracing::instrument(skip(reader))]
pub(crate) async fn read_u64(mut reader: impl AsyncRead + Unpin) -> Result<u64> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf).await.unwrap();
    Ok(u64::from_le_bytes(buf))
}

#[tracing::instrument(skip(writer))]
pub(crate) async fn write_u16(mut writer: impl AsyncWrite + Unpin, value: u16) -> Result<()> {
    writer.write_all(&value.to_be_bytes()).await?;
    Ok(())
}

#[tracing::instrument(skip(writer))]
pub(crate) async fn write_u32(mut writer: impl AsyncWrite + Unpin, value: u32) -> Result<()> {
    writer.write_all(&value.to_be_bytes()).await?;
    Ok(())
}

#[tracing::instrument(skip(writer))]
pub(crate) async fn write_u64(mut writer: impl AsyncWrite + Unpin, value: u64) -> Result<()> {
    writer.write_all(&value.to_be_bytes()).await?;
    Ok(())
}

/// Read and return a dynamic length vector of bytes from a reader which impls AsyncRead.
#[tracing::instrument(skip(reader))]
pub(crate) async fn read_bytes(reader: impl AsyncRead + Unpin, length: usize) -> Result<Vec<u8>> {
    let mut buffer = Vec::with_capacity(length);
    reader.take(length as u64).read_to_end(&mut buffer).await?;
    Ok(buffer)
}
