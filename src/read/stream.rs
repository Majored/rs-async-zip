// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module for reading ZIP file from a non-seekable source.
//!
//! # Example
//! ```
//! ```

use crate::error::{Result, ZipError};
use crate::header::LocalFileHeader;
use crate::read::{CompressionReader, ZipEntry, ZipEntryReader};
use crate::Compression;

use tokio::io::{AsyncRead, AsyncReadExt};

/// A reader which acts over a non-seekable source.
pub struct ZipFileReader<'a, R: AsyncRead + Unpin> {
    pub(crate) reader: &'a mut R,
    pub(crate) entry: Option<ZipEntry>,
    pub(crate) finished: bool,
}

impl<'a, R: AsyncRead + Unpin> ZipFileReader<'a, R> {
    /// Constructs a new ZIP file reader from a mutable reference to a reader.
    pub fn new(reader: &'a mut R) -> Self {
        ZipFileReader { reader, entry: None, finished: false }
    }

    /// Returns whether or not `entry_reader()` will yield more entries.
    pub fn finished(&self) -> bool {
        self.finished
    }

    /// Opens the next entry for reading if the central directory hasn't already been reached.
    pub async fn entry_reader<'b>(&'b mut self) -> Result<Option<ZipEntryReader<'b, R>>> {
        // TODO: Ensure the previous entry has been fully read.

        if self.finished {
            return Ok(None);
        } else if let Some(inner) = read_lfh(self.reader).await? {
            self.entry = Some(inner);
        } else {
            self.finished = true;
            return Ok(None);
        }

        let entry_borrow = self.entry.as_ref().unwrap();

        if entry_borrow.data_descriptor() {
            return Err(ZipError::FeatureNotSupported("Entries with data descriptors"));
        }

        let reader = self.reader.take(entry_borrow.compressed_size.unwrap().into());
        let reader = CompressionReader::from_reader_borrow(entry_borrow.compression(), reader);

        Ok(Some(ZipEntryReader::from_raw(entry_borrow, reader, true)))
    }
}

pub(crate) async fn read_lfh<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Option<ZipEntry>> {
    match reader.read_u32_le().await? {
        crate::delim::LFHD => {}
        crate::delim::CDFHD => return Ok(None),
        actual => return Err(ZipError::UnexpectedHeaderError(actual, crate::delim::LFHD)),
    };

    let header = LocalFileHeader::from_reader(reader).await?;
    let filename = crate::utils::read_string(reader, header.file_name_length.into()).await?;
    let extra = crate::utils::read_bytes(reader, header.extra_field_length.into()).await?;

    let entry = ZipEntry {
        name: filename,
        comment: None,
        data_descriptor: header.flags.data_descriptor,
        crc32: Some(header.crc),
        uncompressed_size: Some(header.uncompressed_size),
        compressed_size: Some(header.compressed_size),
        last_modified: crate::utils::zip_date_to_chrono(header.mod_date, header.mod_time),
        extra: Some(extra),
        compression: Compression::from_u16(header.compression)?,
        offset: None,
    };

    Ok(Some(entry))
}
