// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module for reading ZIP file entries concurrently from an in-memory buffer.

use crate::error::{Result, ZipError};
use crate::read::{CompressionReader, OwnedReader, PrependReader, ZipEntry, ZipEntryReader};
use crate::spec::header::LocalFileHeader;

use std::io::{Cursor, SeekFrom};

use async_io_utilities::AsyncDelimiterReader;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

/// The type returned as an entry reader within this concurrent module.
pub type ConcurrentReader<'b, 'a> = ZipEntryReader<'b, Cursor<&'a [u8]>>;

/// A reader which acts concurrently over an in-memory buffer.
pub struct ZipFileReader<'a> {
    pub(crate) data: &'a [u8],
    pub(crate) entries: Vec<ZipEntry>,
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
        cursor.seek(SeekFrom::Start(entry.offset.unwrap() as u64 + 4)).await?;

        let header = LocalFileHeader::from_reader(&mut cursor).await?;
        let data_offset = (header.file_name_length + header.extra_field_length) as i64;
        cursor.seek(SeekFrom::Current(data_offset)).await?;

        if entry.data_descriptor() {
            let delimiter = crate::spec::signature::DATA_DESCRIPTOR.to_le_bytes();
            let reader = OwnedReader::Owned(cursor);
            let reader = PrependReader::Normal(reader);
            //let reader = AsyncDelimiterReader::new(reader, &delimiter);
            let reader = CompressionReader::from_reader(entry.compression(), reader);

            Ok(ZipEntryReader::from_raw(entry, reader, true))
        } else {
            let reader = OwnedReader::Owned(cursor);
            let reader = PrependReader::Normal(reader);
            let reader =
                CompressionReader::from_reader_take(entry.compression(), reader, entry.compressed_size.unwrap().into());

            Ok(ZipEntryReader::from_raw(entry, reader, false))
        }
    }
}
