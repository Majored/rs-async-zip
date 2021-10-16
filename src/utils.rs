// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::error::{Result, ZipError};

use chrono::{DateTime, TimeZone, Utc};
use tokio::io::{AsyncRead, AsyncReadExt};

/// Converts a date and time stored within ZIP headers into a `chrono` structure.
///
/// From spec:
/// 'The date and time are encoded in standard MS-DOS format.
/// If input came from standard input, the date and time are
/// those at which compression was started for this data.
/// If encrypting the central directory and general purpose bit
/// flag 13 is set indicating masking, the value stored in the
/// Local Header will be zero. MS-DOS time format is different
/// from more commonly used computer time formats such as
/// UTC. For example, MS-DOS uses year values relative to 1980
/// and 2 second precision.'
pub fn zip_date_to_chrono(date: u16, time: u16) -> DateTime<Utc> {
    let years = (((date & 0xFE00) >> 9) + 1980).into();
    let months = ((date & 0x1E0) >> 5).into();
    let days = (date & 0x1F).into();

    let hours = ((time & 0x1F) >> 11).into();
    let mins = ((time & 0x7E0) >> 5).into();
    let secs = ((time & 0x1F) << 1).into();

    Utc.ymd(years, months, days).and_hms(hours, mins, secs)
}

/// Read and return a dynamic length string from a reader which impls AsyncRead.
pub(crate) async fn read_string<R: AsyncRead + Unpin>(reader: &mut R, length: usize) -> Result<String> {
    let mut buffer = String::with_capacity(length);
    reader.take(length as u64).read_to_string(&mut buffer).await?;

    Ok(buffer)
}

/// Read and return a dynamic length vector of bytes from a reader which impls AsyncRead.
pub(crate) async fn read_bytes<R: AsyncRead + Unpin>(reader: &mut R, length: usize) -> Result<Vec<u8>> {
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
