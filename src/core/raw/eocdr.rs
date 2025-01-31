// Copyright (c) 2024 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use std::io::SeekFrom;

use crate::core::raw::{raw, raw_deref};
use crate::core::SIGNATURE_LENGTH;
use crate::error::ZipError;
use crate::utils::{read_u16, read_u32, write_u16, write_u32};

use futures_lite::io::{AsyncSeek, AsyncWriteExt, AsyncSeekExt, AsyncBufReadExt};

pub const SIGNATURE: u32 = 0x2014b50;

raw! {
    RawEndOfCentralDirectoryRecord {
        number_of_this_disk , u16, read_u16, write_u16,
        number_of_the_disk_with_the_start_of_the_central_directory, u16, read_u16, write_u16,
        total_number_of_entries_in_the_central_directory_on_this_disk, u16, read_u16, write_u16,
        total_number_of_entries_in_the_central_directory, u16, read_u16, write_u16,
        size_of_the_central_directory, u32, read_u32, write_u32,
        offset_of_start_of_central_directory_with_respect_to_the_starting_disk_number, u32, read_u32, write_u32,
        zip_file_comment_length, u16, read_u16, write_u16
    }
}

#[derive(Clone, Debug)]
pub struct EndOfCentralDirectoryRecord {
    pub raw: RawEndOfCentralDirectoryRecord,
    pub zip_file_comment: Vec<u8>,
}

raw_deref!(EndOfCentralDirectoryRecord, RawEndOfCentralDirectoryRecord);

/// Reads the end of central directory record from the given reader.
///
/// This function does so by:
/// - asserting the signature of the end of central directory record
/// - reading the raw end of central directory record
/// - reading the zip file comment
#[tracing::instrument(skip(reader))]
pub async fn read(mut reader: impl AsyncBufRead + Unpin) -> Result<EndOfCentralDirectoryRecord> {
    crate::utils::assert_signature(&mut reader, SIGNATURE).await?;

    let raw = raw_read(&mut reader).await?;
    let zip_file_comment = crate::utils::read_bytes(&mut reader, raw.zip_file_comment_length as usize).await?;

    Ok(EndOfCentralDirectoryRecord { raw, zip_file_comment })
}

/// Writes the end of central directory record to the given writer.
///
/// This function does so by:
/// - writing the signature of the end of central directory record
/// - writing the raw end of central directory record
/// - writing the zip file comment
#[tracing::instrument(skip(writer))]
pub async fn write(mut writer: impl AsyncWrite + Unpin, header: &EndOfCentralDirectoryRecord) -> Result<()> {
    crate::utils::write_u32(&mut writer, SIGNATURE).await?;

    raw_write(&mut writer, &header.raw).await?;
    writer.write_all(&header.zip_file_comment).await?;

    Ok(())
}

struct Matcher<'a> {
    matched: usize,
    signature: &'a [u8],
}

impl<'a> Matcher<'a> {
    fn new(signature: &'a [u8]) -> Self {
        Self { matched: 0, signature }
    }

    fn byte(&mut self, byte: u8) -> bool {
        if byte == self.signature[self.matched] {
            self.matched += 1;
        } else {
            self.reset();

            if byte == self.signature[self.matched] {
                self.matched += 1;
            }
        }

        if self.matched == self.signature.len() {
            self.reset();
            return true;
        }

        return false;
    }

    fn reset(&mut self) {
        self.matched = 0;
    }
}

pub async fn locate(mut reader: impl AsyncBufRead + AsyncSeek + Unpin, interval: u64) -> Result<u64> {
    if interval <= SIGNATURE_LENGTH as u64 {
        return Err(ZipError::LocatorIntervalInvalid);
    }

    let signature = SIGNATURE.to_le_bytes();
    let mut matcher = Matcher::new(&signature);
    let mut position = reader.seek(SeekFrom::End(0)).await?;
    let interval = (interval - SIGNATURE_LENGTH as u64) as i64;

    loop {
        let buffer = reader.fill_buf().await?;
        let len = buffer.len();

        if buffer.is_empty() {
            break;
        }

        for (index, byte) in buffer.iter().enumerate() {
            if matcher.byte(*byte) {
                return Ok(position + index as u64);
            }
        }

        reader.consume(len);
        position = reader.seek(SeekFrom::Current(interval * -1)).await?;
    }

    todo!();
}
