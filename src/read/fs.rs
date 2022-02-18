// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module for reading ZIP file entries concurrently from the filesystem.
//!
//! # Example
//! ```no_run
//! # use async_zip::read::fs::ZipFileReader;
//! # use async_zip::error::ZipError;
//! #
//! # async fn run() -> Result<(), ZipError> {
//! let zip = ZipFileReader::new(String::from("./Archive.zip")).await.unwrap();
//! assert_eq!(zip.entries().len(), 2);
//!
//! let mut reader1 = zip.entry_reader(0).await.unwrap();
//! let mut reader2 = zip.entry_reader(1).await.unwrap();
//!
//! tokio::select! {
//!    _ = reader1.read_to_string_crc() => {}
//!    _ = reader2.read_to_string_crc() => {}
//! };
//! #   Ok(())
//! # }
//! ```

use super::CompressionReader;
use crate::error::{Result, ZipError};
use crate::read::{ZipEntry, ZipEntryReader, OwnedReader, PrependReader};

use std::io::SeekFrom;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

/// A reader which acts concurrently over a filesystem file.
pub struct ZipFileReader {
    pub(crate) filename: String,
    pub(crate) entries: Vec<ZipEntry>,
}

impl ZipFileReader {
    /// Constructs a new ZIP file reader from a filename.
    pub async fn new(filename: String) -> Result<ZipFileReader> {
        let mut fs_file = File::open(&filename).await?;
        let entries = crate::read::seek::read_cd(&mut fs_file).await?;

        Ok(ZipFileReader { filename, entries })
    }

    crate::read::reader_entry_impl!();

    /// Opens an entry at the provided index for reading.
    pub async fn entry_reader(&self, index: usize) -> Result<ZipEntryReader<'_, File>> {
        let entry = self.entries.get(index).ok_or(ZipError::EntryIndexOutOfBounds)?;

        if entry.data_descriptor() {
            return Err(ZipError::FeatureNotSupported("Entries with data descriptors"));
        }

        let mut fs_file = File::open(&self.filename).await?;
        fs_file.seek(SeekFrom::Start(entry.data_offset())).await?;

        let reader = OwnedReader::Owned(fs_file);
        let reader = PrependReader::Normal(reader);
        let reader = reader.take(entry.compressed_size.unwrap().into());
        let reader = CompressionReader::from_reader(entry.compression(), reader);

        Ok(ZipEntryReader::from_raw(entry, reader, false))
    }
}
