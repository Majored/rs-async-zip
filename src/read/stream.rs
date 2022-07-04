// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module for reading ZIP files from a non-seekable source.
//! 
//! ## Note
//! This method fully relies on the information provided in each individual local file header. As a result, this method
//! doesn't support entries which desynchronise the information provided in the local file header VS the central
//! directory file header. This is a practice that the 
//! [specification suggests](https://github.com/Majored/rs-async-zip/blob/main/SPECIFICATION.md#719) when you wish to
//! conceal that information.
//! 
//! ## Example
//! ```no_run
//! # use async_zip::read::stream::ZipFileReader;
//! # use tokio::fs::File;
//! # use async_zip::error::ZipError;
//! #
//! # async fn run() -> Result<(), ZipError> {
//! let mut file = File::open("./Archive.zip").await?;
//! let mut zip = ZipFileReader::new(&mut file);
//!
//! // Consume all entries in order.
//! while !zip.finished() {
//!     if let Some(reader) = zip.entry_reader().await? {
//!         reader.read_to_string_crc().await?;
//!     }
//! }
//! #   Ok(())
//! # }
//! ```

use crate::error::{Result, ZipError};
use crate::read::{CompressionReader, OwnedReader, PrependReader, ZipEntry, ZipEntryReader};
use crate::spec::compression::Compression;
use crate::spec::header::LocalFileHeader;

use async_io_utilities::AsyncPrependReader;
use tokio::io::{AsyncRead, AsyncReadExt};

/// A reader which acts over a non-seekable source.
pub struct ZipFileReader<R: AsyncRead + Unpin> {
    pub(crate) reader: AsyncPrependReader<R>,
    pub(crate) entry: Option<ZipEntry>,
    pub(crate) finished: bool,
}

impl<R: AsyncRead + Unpin> ZipFileReader<R> {
    /// Constructs a new ZIP file reader from a reader which implements [`AsyncRead`].
    pub fn new(reader: R) -> Self {
        let reader = AsyncPrependReader::new(reader);
        ZipFileReader { reader, entry: None, finished: false }
    }

    /// Returns whether or not it's possible for this reader to yeild more entries.
    /// 
    /// # Note
    /// It's still possible for this function to return false whilst a call to [`ZipFileReader::entry_reader()`]
    /// returns no entry. This happens in the case where the last entry has been read but the central directory has not
    /// been reached (and would be reached in a succeeding call to [`ZipFileReader::entry_reader()`]).
    pub fn finished(&self) -> bool {
        self.finished
    }

    /// Opens the next entry for reading if the central directory hasn't yet been reached.
    /// 
    /// # Note
    /// It's essential that each entry reader returned by this function is fully consumed before a new one is opened.
    pub async fn entry_reader(&mut self) -> Result<Option<ZipEntryReader<'_, R>>> {
        if self.finished {
            return Ok(None);
        } else if let Some(inner) = read_lfh(&mut self.reader).await? {
            self.entry = Some(inner);
        } else {
            self.finished = true;
            return Ok(None);
        }

        let entry_borrow = self.entry.as_ref().unwrap();

        let reader = OwnedReader::Borrow(&mut self.reader);
        let reader = PrependReader::Prepend(reader);
        let reader = CompressionReader::from_reader(
            entry_borrow.compression(),
            reader,
            entry_borrow.compressed_size.map(u32::into),
        );

        Ok(Some(ZipEntryReader::from_raw(entry_borrow, reader, entry_borrow.data_descriptor())))
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
