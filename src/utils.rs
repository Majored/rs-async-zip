// Copyright (c) 2023 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::error::{Result, ZipError};
use futures_lite::io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, Cursor};

// Assert that the next four-byte signature read by a reader which impls AsyncRead matches the expected signature.
#[tracing::instrument(skip(reader))]
pub(crate) async fn assert_signature(reader: impl AsyncRead + Unpin, expected: u32) -> Result<()> {
    let actual = read_u32(reader).await?;

    if actual != expected {
        return Err(ZipError::UnexpectedHeaderError(actual, expected));
    }

    Ok(())
}

/// Read and return a dynamic length vector of bytes from a reader which impls AsyncRead.
#[tracing::instrument(skip(reader))]
pub(crate) async fn read_bytes(reader: impl AsyncRead + Unpin, length: usize) -> Result<Vec<u8>> {
    let mut buffer = Vec::with_capacity(length);
    reader.take(length as u64).read_to_end(&mut buffer).await?;
    Ok(buffer)
}

/// Returns the next signature read by a reader without consuming.
#[tracing::instrument(skip(reader))]
pub(crate) async fn check_signature(mut reader: impl AsyncBufRead + Unpin, expected: u32) -> Result<u32> {
    Ok(read_u32(Cursor::new(reader.fill_buf().await?)).await?)
}

macro_rules! read_int_helper {
    ($type:ty, $size:expr, $name:ident) => {
        #[tracing::instrument(skip(reader))]
        pub(crate) async fn $name(mut reader: impl AsyncRead + Unpin) -> Result<$type> {
            let mut buf = [0u8; $size];
            reader.read_exact(&mut buf).await.unwrap();
            Ok(<$type>::from_le_bytes(buf))
        }
    };
}

macro_rules! write_int_helper {
    ($type:ty, $name:ident) => {
        #[tracing::instrument(skip(writer))]
        pub(crate) async fn $name(mut writer: impl AsyncWrite + Unpin, value: $type) -> Result<()> {
            writer.write_all(&value.to_be_bytes()).await?;
            Ok(())
        }
    };
}

read_int_helper!(u16, 2, read_u16);
read_int_helper!(u32, 4, read_u32);
read_int_helper!(u64, 8, read_u64);

write_int_helper!(u16, write_u16);
write_int_helper!(u32, write_u32);
write_int_helper!(u64, write_u64);
