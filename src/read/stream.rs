// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module for reading ZIP file from a non-seekable source.
//!
//! # Example
//! ```
//! ```

use crate::error::{Result, ZipError};
use crate::read::{CompressionReader, OwnedReader, PrependReader, ZipEntry, ZipEntryReader};
use crate::spec::compression::Compression;
use crate::spec::header::LocalFileHeader;

use async_io_utilities::{AsyncDelimiterReader, AsyncPrependReader};
use tokio::io::{AsyncRead, AsyncReadExt};

/// A reader which acts over a non-seekable source.
pub struct ZipFileReader<R: AsyncRead + Unpin> {
    pub(crate) reader: AsyncPrependReader<R>,
    pub(crate) entry: Option<ZipEntry>,
    pub(crate) finished: bool,
}

impl<R: AsyncRead + Unpin> ZipFileReader<R> {
    /// Constructs a new ZIP file reader from a mutable reference to a reader.
    pub fn new(reader: R) -> Self {
        let reader = AsyncPrependReader::new(reader);
        ZipFileReader { reader, entry: None, finished: false }
    }

    /// Returns whether or not `entry_reader()` will yield more entries.
    pub fn finished(&self) -> bool {
        self.finished
    }

    /// Opens the next entry for reading if the central directory hasn't already been reached.
    pub async fn entry_reader(&mut self) -> Result<Option<ZipEntryReader<'_, R>>> {
        // TODO: Ensure the previous entry has been fully read.

        if self.finished {
            return Ok(None);
        } else if let Some(inner) = read_lfh(&mut self.reader).await? {
            self.entry = Some(inner);
        } else {
            self.finished = true;
            return Ok(None);
        }

        let entry_borrow = self.entry.as_ref().unwrap();

        if entry_borrow.data_descriptor() {
            let delimiter = crate::spec::signature::DATA_DESCRIPTOR.to_le_bytes();
            let reader = OwnedReader::Borrow(&mut self.reader);
            let reader = PrependReader::Prepend(reader);
            let reader = CompressionReader::from_reader(entry_borrow.compression(), reader);

            Ok(Some(ZipEntryReader::from_raw(entry_borrow, reader, true)))
        } else {
            let reader = OwnedReader::Borrow(&mut self.reader);
            let reader = PrependReader::Prepend(reader);
            let reader = CompressionReader::from_reader_take(
                entry_borrow.compression(),
                reader,
                entry_borrow.compressed_size.unwrap().into(),
            );

            Ok(Some(ZipEntryReader::from_raw(entry_borrow, reader, true)))
        }
    }
}

pub(crate) async fn read_lfh<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Option<ZipEntry>> {
    match reader.read_u32_le().await? {
        crate::spec::signature::LOCAL_FILE_HEADER => {}
        crate::spec::signature::CENTRAL_DIRECTORY_FILE_HEADER => return Ok(None),
        actual => return Err(ZipError::UnexpectedHeaderError(actual, crate::spec::signature::LOCAL_FILE_HEADER)),
    };

    let header = LocalFileHeader::from_reader(reader).await?;
    let filename = async_io_utilities::read_string(reader, header.file_name_length.into()).await?;
    let extra = async_io_utilities::read_bytes(reader, header.extra_field_length.into()).await?;

    let entry = ZipEntry {
        name: filename,
        comment: None,
        data_descriptor: header.flags.data_descriptor,
        crc32: Some(header.crc),
        uncompressed_size: Some(header.uncompressed_size),
        compressed_size: Some(header.compressed_size),
        last_modified: crate::spec::date::zip_date_to_chrono(header.mod_date, header.mod_time),
        extra: Some(extra),
        compression: Compression::from_u16(header.compression)?,
        offset: None,
    };

    Ok(Some(entry))
}
