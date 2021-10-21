// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module for reading ZIP file entries concurrently from the filesystem.
//!
//! # Example
//! ```
//! let zip = ZipFileReader::new("./Archive.zip").await.unwrap();
//!
//! assert_eq!(zip.entries().len(), 2);
//!
//! let mut reader1 = zip.entry_reader(0).await.unwrap();
//! let mut reader2 = zip.entry_reader(1).await.unwrap();
//!
//! let mut buff1 = String::new();
//! let mut buff2 = String::new();
//!
//! tokio::select! {
//!     _ = reader1.read_to_string(&mut buff1) => {}
//!     _ = reader2.read_to_string(&mut buff2) => {}
//! };
//! ```

use super::CompressionReader;
use crate::error::{Result, ZipError};
use crate::read::{ZipEntry, ZipEntryReader};

use std::io::SeekFrom;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

/// The type returned as an entry reader within this concurrent module.
pub type ConcurrentReader<'a> = ZipEntryReader<'a, File>;

/// A reader which acts concurrently over a filesystem file.
pub struct ZipFileReader<'a> {
    pub(crate) filename: &'a str,
    pub(crate) entries: Vec<ZipEntry>,
}

impl<'a> ZipFileReader<'a> {
    /// Constructs a new ZIP file reader from a filename.
    pub async fn new(filename: &'a str) -> Result<ZipFileReader<'a>> {
        let mut fs_file = File::open(filename).await?;
        let entries = crate::read::seek::read_cd(&mut fs_file).await?;

        Ok(ZipFileReader { filename, entries })
    }

    crate::read::reader_entry_impl!();

    /// Opens an entry at the provided index for reading.
    pub async fn entry_reader(&self, index: usize) -> Result<ConcurrentReader<'_>> {
        let entry = self.entries.get(index).ok_or(ZipError::EntryIndexOutOfBounds)?;

        let mut fs_file = File::open(self.filename).await?;
        fs_file.seek(SeekFrom::Start(entry.data_offset())).await?;

        let reader = fs_file.take(entry.compressed_size.unwrap().into());
        let reader = CompressionReader::from_reader(entry.compression(), reader);

        Ok(ZipEntryReader::from_raw(entry, reader, false))
    }
}
