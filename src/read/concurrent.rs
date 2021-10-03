// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use super::CompressionReader;
use crate::error::{Result, ZipError};
use crate::read::{ZipEntry, ZipEntryReader};

use std::io::SeekFrom;
use tokio::fs::File;
use tokio::io::AsyncSeekExt;

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
        for index in 0..self.entries.len() {
            let current_entry = self.entries.get(index).unwrap();
            
            if current_entry.name() == name {
                return Some((index, current_entry));
            }
        }

        None
    }

    /// Opens an entry at the provided index for reading.
    pub async fn entry_reader(&self, index: usize) -> Result<ZipEntryReader<'_, File>> {
        let entry = self.entries.get(index).ok_or(ZipError::EntryIndexOutOfBounds)?;

        let mut fs_file = File::open(self.filename).await?;
        fs_file.seek(SeekFrom::Start(entry.data_offset())).await?;
        let reader = CompressionReader::from_reader(entry.compression(), fs_file);

        Ok(ZipEntryReader { entry, reader })
    }
}
