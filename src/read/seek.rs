// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module for reading ZIP file from a seekable source.
//! 
//! # Example
//! ```
//! let mut file = File::open("./Archive.zip").await.unwrap();
//! let mut zip = ZipFileReader::new(&mut file).await.unwrap();
//! 
//! assert_eq!(zip.entries().len(), 2);
//! 
//! // Consume the entries out-of-order.
//! let mut reader = zip.entry_reader(1).await.unwrap();
//! reader.read_to_string(&mut String::new()).await.unwrap();
//! 
//! let mut reader = zip.entry_reader(0).await.unwrap();
//! reader.read_to_string(&mut String::new()).await.unwrap();
//! ```

use crate::error::{Result, ZipError};
use crate::header::{CentralDirectoryHeader, EndOfCentralDirectoryHeader};
use crate::read::{ZipEntry, ZipEntryReader, CompressionReader};

use tokio::io::{AsyncRead, AsyncSeek, AsyncSeekExt};

use std::io::SeekFrom;

/// A reader which acts over a seekable source.
pub struct ZipFileReader<'a, R: AsyncRead + AsyncSeek + Unpin> {
    pub(crate) reader: &'a mut R,
    pub(crate) entries: Vec<ZipEntry>,
}

impl<'a, R: AsyncRead + AsyncSeek + Unpin> ZipFileReader<'a, R> {
    /// Constructs a new ZIP file reader from a mutable reference to a reader.
    pub async fn new(reader: &'a mut R) -> Result<ZipFileReader<'a, R>> {
        let entries = read_cd(reader).await?;
        Ok(ZipFileReader { reader, entries })
    }

    /// Returns a shared reference to a list of the ZIP file's entries.
    pub fn entries(&self) -> &Vec<ZipEntry> {
        &self.entries
    }

    /// Searches for an entry with a specific filename.
    pub fn entry(&self, name: &str) -> Option<(usize, &ZipEntry)> {
        for (index, entry) in self.entries().iter().enumerate() {
            if entry.name() == name {
                return Some((index, entry));
            }
        }

        None
    }

    /// Opens an entry at the provided index for reading.
    pub async fn entry_reader<'b>(&'b mut self, index: usize) -> Result<ZipEntryReader<'b, R>> {
        let entry = self.entries.get(index).ok_or(ZipError::EntryIndexOutOfBounds)?;

        self.reader.seek(SeekFrom::Start(entry.data_offset())).await?;
        let reader = CompressionReader::from_reader_borrow(entry.compression(), self.reader);

        Ok(ZipEntryReader { entry, reader })
    }
}

pub(crate) async fn read_cd<R: AsyncRead + AsyncSeek + Unpin>(reader: &mut R) -> Result<Vec<ZipEntry>> {
    // Assume no ZIP comment exists for the moment so we can seek directly to EOCD header.
    reader.seek(SeekFrom::End(-22)).await?;

    if super::read_u32(reader).await? != crate::delim::EOCDD {
        return Err(ZipError::FeatureNotSupported("ZIP file comment"));
    }

    let eocdh = EndOfCentralDirectoryHeader::from_reader(reader).await?;

    // Outdated feature so unlikely to ever make it into this crate.
    if eocdh.disk_num != eocdh.start_cent_dir_disk || eocdh.num_of_entries != eocdh.num_of_entries_disk {
        return Err(ZipError::FeatureNotSupported("Spanned/split files"));
    }

    reader.seek(SeekFrom::Start(eocdh.cent_dir_offset.into())).await?;
    let mut entries = Vec::with_capacity(eocdh.num_of_entries.into());

    for _ in 0..eocdh.num_of_entries {
        if super::read_u32(reader).await? != crate::delim::CDFHD {
            return Err(ZipError::ReadFailed); // Alter error message.
        }

        // Ignore file extra & comment for the moment.
        let header = CentralDirectoryHeader::from_reader(reader).await?;
        let filename = super::read_string(reader, header.file_name_length).await?;
        reader.seek(SeekFrom::Current((header.extra_field_length + header.file_comment_length).into())).await?;

        entries.push(ZipEntry::from_raw(header, filename)?);
    }

    Ok(entries)
}
