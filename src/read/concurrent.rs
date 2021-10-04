// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module for reading ZIP file entries concurrently from the filesystem.
//!
//! # Note
//! To enable concurrency, this module's ZipFileReader will open a new file for each call to `entry_reader()` and seek
//! to the relevant entry's data offset. Thus, any caller needs to be aware that for large ZIP files with many entries,
//! you may hit an OS file limit if attempting to open all entries concurrently. To mitigate this, either:
//! - Increase the execeuting user's file limit (often via the 'ulimit' command).
//! - Or; only process a set number of entries at any one time.
//! 
//! # Example

use super::CompressionReader;
use crate::error::{Result, ZipError};
use crate::read::{ZipEntry, ZipEntryReader};

use std::io::SeekFrom;
use tokio::fs::File;
use tokio::io::{Take, AsyncSeekExt, AsyncReadExt};

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

    /// Returns a shared reference to a list of the ZIP file's entries.
    pub fn entries(&self) -> &Vec<ZipEntry> {
        &self.entries
    }

    /// Searches for an entry with a specific filename.
    /// 
    /// If an entry is found, a tuple containing the index it was found at, as well as a shared reference to the
    /// ZipEntry itself is returned. Else, None is returned.
    pub fn entry(&self, name: &str) -> Option<(usize, &ZipEntry)> {
        for (index, entry) in self.entries().iter().enumerate() {
            if entry.name() == name {
                return Some((index, entry));
            }
        }

        None
    }

    /// Opens an entry at the provided index for reading.
    pub async fn entry_reader(&self, index: usize) -> Result<ZipEntryReader<'_, Take<File>>> {
        let entry = self.entries.get(index).ok_or(ZipError::EntryIndexOutOfBounds)?;

        let mut fs_file = File::open(self.filename).await?;
        fs_file.seek(SeekFrom::Start(entry.data_offset())).await?;
        let reader = fs_file.take(entry.uncompressed_size.unwrap().into());
        let reader = CompressionReader::from_reader(entry.compression(), reader);

        Ok(ZipEntryReader { entry, reader })
    }
}
