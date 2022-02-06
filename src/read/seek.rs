// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module for reading ZIP file from a seekable source.
//!
//! # Example
//! ```no_run
//! # use async_zip::read::seek::ZipFileReader;
//! # use tokio::fs::File;
//! # use async_zip::error::ZipError;
//! #
//! # async fn run() -> Result<(), ZipError> {
//! let mut file = File::open("./Archive.zip").await.unwrap();
//! let mut zip = ZipFileReader::new(&mut file).await?;
//!
//! assert_eq!(zip.entries().len(), 2);
//!
//! // Consume the entries out-of-order.
//! let mut reader = zip.entry_reader(1).await?;
//! reader.read_to_string_crc().await?;
//!
//! let mut reader = zip.entry_reader(0).await?;
//! reader.read_to_string_crc().await?;
//! #   Ok(())
//! # }
//! ```

use crate::error::{Result, ZipError};
use crate::read::{CompressionReader, ZipEntry, ZipEntryReader};
use crate::spec::compression::Compression;
use crate::spec::header::{CentralDirectoryHeader, EndOfCentralDirectoryHeader};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt};

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

    crate::read::reader_entry_impl!();

    /// Opens an entry at the provided index for reading.
    pub async fn entry_reader<'b>(&'b mut self, index: usize) -> Result<ZipEntryReader<'b, R>> {
        let entry = self.entries.get(index).ok_or(ZipError::EntryIndexOutOfBounds)?;

        if entry.data_descriptor() {
            return Err(ZipError::FeatureNotSupported("Entries with data descriptors"));
        }

        self.reader.seek(SeekFrom::Start(entry.data_offset())).await?;

        let reader = self.reader.take(entry.compressed_size.unwrap().into());
        let reader = CompressionReader::from_reader_borrow(entry.compression(), reader);

        Ok(ZipEntryReader::from_raw(entry, reader, false))
    }
}

pub(crate) async fn read_cd<R: AsyncRead + AsyncSeek + Unpin>(reader: &mut R) -> Result<Vec<ZipEntry>> {
    // Assume no ZIP comment exists for the moment so we can seek directly to EOCD header.
    reader.seek(SeekFrom::End(-22)).await?;
    crate::utils::assert_delimiter(reader, crate::spec::delimiter::EOCDD).await?;

    let eocdh = EndOfCentralDirectoryHeader::from_reader(reader).await?;

    // Outdated feature so unlikely to ever make it into this crate.
    if eocdh.disk_num != eocdh.start_cent_dir_disk || eocdh.num_of_entries != eocdh.num_of_entries_disk {
        return Err(ZipError::FeatureNotSupported("Spanned/split files"));
    }

    reader.seek(SeekFrom::Start(eocdh.cent_dir_offset.into())).await?;
    let mut entries = Vec::with_capacity(eocdh.num_of_entries.into());

    for _ in 0..eocdh.num_of_entries {
        entries.push(read_cd_entry(reader).await?);
    }

    Ok(entries)
}

pub(crate) async fn read_cd_entry<R: AsyncRead + Unpin>(reader: &mut R) -> Result<ZipEntry> {
    crate::utils::assert_delimiter(reader, crate::spec::delimiter::CDFHD).await?;

    let header = CentralDirectoryHeader::from_reader(reader).await?;
    let filename = crate::utils::read_string(reader, header.file_name_length.into()).await?;
    let extra = crate::utils::read_bytes(reader, header.extra_field_length.into()).await?;
    let comment = crate::utils::read_string(reader, header.file_comment_length.into()).await?;

    let entry = ZipEntry {
        name: filename,
        comment: Some(comment),
        data_descriptor: header.flags.data_descriptor,
        crc32: Some(header.crc),
        uncompressed_size: Some(header.uncompressed_size),
        compressed_size: Some(header.compressed_size),
        last_modified: crate::utils::zip_date_to_chrono(header.mod_date, header.mod_time),
        extra: Some(extra),
        compression: Compression::from_u16(header.compression)?,
        offset: Some(header.lh_offset),
    };

    Ok(entry)
}
