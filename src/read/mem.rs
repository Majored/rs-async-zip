// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module for reading ZIP file entries concurrently from an in-memory buffer.

use crate::error::{Result, ZipError};
use crate::read::{CompressionReader, OwnedReader, PrependReader, ZipEntry, ZipEntryReader};
use crate::spec::header::LocalFileHeader;
use crate::read::ZipEntryMeta;

use std::io::{Cursor, SeekFrom};

use tokio::io::AsyncSeekExt;

/// The type returned as an entry reader within this concurrent module.
pub type ConcurrentReader<'b, 'a> = ZipEntryReader<'b, Cursor<&'a [u8]>>;

/// A reader which acts concurrently over an in-memory buffer.
pub struct ZipFileReader<'a> {
    pub(crate) data: &'a [u8],
    pub(crate) entries: Vec<(ZipEntry, ZipEntryMeta)>,
    pub(crate) comment: Option<String>,
}

impl<'a> ZipFileReader<'a> {
    /// Constructs a new ZIP file reader from an in-memory buffer.
    pub async fn new(data: &'a [u8]) -> Result<ZipFileReader<'a>> {
        let (entries, comment) = crate::read::seek::read_cd(&mut Cursor::new(data)).await?;
        Ok(ZipFileReader { data, entries, comment })
    }

    crate::read::reader_entry_impl!();

    /// Opens an entry at the provided index for reading.
    pub async fn entry_reader<'b>(&'b mut self, index: usize) -> Result<ConcurrentReader<'b, 'a>> {
        let entry = self.entries.get(index).ok_or(ZipError::EntryIndexOutOfBounds)?;

        let mut cursor = Cursor::new(<&[u8]>::clone(&self.data));
        cursor.seek(SeekFrom::Start(entry.1.file_offset.unwrap() as u64 + 4)).await?;

        let header = LocalFileHeader::from_reader(&mut cursor).await?;
        let data_offset = (header.file_name_length + header.extra_field_length) as i64;
        cursor.seek(SeekFrom::Current(data_offset)).await?;

        let reader = OwnedReader::Owned(cursor);
        let reader = PrependReader::Normal(reader);
        let reader = CompressionReader::from_reader(&entry.0.compression(), reader, Some(entry.0.compressed_size()).map(u32::into))?;

        Ok(ZipEntryReader::from_raw(&entry.0, &entry.1, reader, entry.1.general_purpose_flag.data_descriptor))
    }
}
