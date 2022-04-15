// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::error::{Result, ZipError};
use tokio::io::{AsyncRead, AsyncReadExt};

// Assert that the next four-byte signature read by a reader which impls AsyncRead matches the expected signature.
pub(crate) async fn assert_signature<R: AsyncRead + Unpin>(reader: &mut R, expected: u32) -> Result<()> {
    match reader.read_u32_le().await? {
        actual if actual == expected => Ok(()),
        actual => Err(ZipError::UnexpectedHeaderError(actual, expected)),
    }
}
