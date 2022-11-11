// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

pub(crate) mod compressed;
pub(crate) mod hashed;
pub(crate) mod owned;
pub(crate) mod entry;
pub(crate) mod locator;

use tokio::io::{AsyncRead, AsyncReadExt};

/// Read and return a dynamic length string from a reader which impls AsyncRead.
pub(crate) async fn read_string<R: AsyncRead + Unpin>(reader: R, length: usize) -> std::io::Result<String> {
    let mut buffer = String::with_capacity(length);
    reader.take(length as u64).read_to_string(&mut buffer).await?;
    
    Ok(buffer)
}

/// Read and return a dynamic length vector of bytes from a reader which impls AsyncRead.
pub(crate) async fn read_bytes<R: AsyncRead + Unpin>(reader: R, length: usize) -> std::io::Result<Vec<u8>> {
    let mut buffer = Vec::with_capacity(length);
    reader.take(length as u64).read_to_end(&mut buffer).await?;

    Ok(buffer)
}

/// A macro that returns the inner value of an Ok or early-returns in the case of an Err.
/// 
/// This is almost identical to the ? operator but handles the situation when a Result is used in combination with
/// Poll (eg. tokio's IO traits such as AsyncRead).
macro_rules! poll_result_ok {
    ($poll:expr) => {
        match $poll {
            Ok(inner) => inner,
            Err(err) => return Poll::Ready(Err(err)),
        }
    };
}

use poll_result_ok;