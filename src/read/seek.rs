// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module for reading ZIP files from a seekable source.
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
use crate::read::{CompressionReader, OwnedReader, PrependReader, ZipEntry, ZipEntryReader};
use crate::spec::compression::Compression;
use crate::spec::header::{CentralDirectoryHeader, EndOfCentralDirectoryHeader, LocalFileHeader};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use async_io_utilities::AsyncDelimiterReader;
use std::io::SeekFrom;

/// A reader which acts over a seekable source.
pub struct ZipFileReader<R: AsyncRead + AsyncSeek + Unpin> {
    pub(crate) reader: R,
    pub(crate) entries: Vec<ZipEntry>,
    pub(crate) comment: Option<String>,
}

impl<R: AsyncRead + AsyncSeek + Unpin> ZipFileReader<R> {
    /// Constructs a new ZIP file reader from a reader which implements [`AsyncRead`] and [`AsyncSeek`].
    pub async fn new(mut reader: R) -> Result<ZipFileReader<R>> {
        let (entries, comment) = read_cd(&mut reader).await?;
        Ok(ZipFileReader { reader, entries, comment })
    }

    crate::read::reader_entry_impl!();

    /// Opens an entry at the provided index for reading.
    pub async fn entry_reader(&mut self, index: usize) -> Result<ZipEntryReader<'_, R>> {
        let entry = self.entries.get(index).ok_or(ZipError::EntryIndexOutOfBounds)?;

        self.reader.seek(SeekFrom::Start(entry.offset.unwrap() as u64 + 4)).await?;

        let header = LocalFileHeader::from_reader(&mut self.reader).await?;
        let data_offset = (header.file_name_length + header.extra_field_length) as i64;
        self.reader.seek(SeekFrom::Current(data_offset)).await?;

        let reader = OwnedReader::Borrow(&mut self.reader);
        let reader = PrependReader::Normal(reader);
        let reader = CompressionReader::from_reader(entry.compression(), reader, entry.compressed_size.map(u32::into));

        Ok(ZipEntryReader::from_raw(entry, reader, entry.data_descriptor()))
    }
}

pub(crate) async fn read_cd<R: AsyncRead + AsyncSeek + Unpin>(
    reader: &mut R,
) -> Result<(Vec<ZipEntry>, Option<String>)> {
    const MAX_ENDING_LENGTH: u64 = u16::MAX as u64 + 22;

    let length = reader.seek(SeekFrom::End(0)).await?;
    let seek_to = length.saturating_sub(MAX_ENDING_LENGTH);

    reader.seek(SeekFrom::Start(seek_to)).await?;

    let mut matched_offset: Option<u64> = None;
    let mut comment = None;
    let delimiter = crate::spec::signature::END_OF_CENTRAL_DIRECTORY.to_le_bytes();
    let mut reader = AsyncDelimiterReader::new(reader, &delimiter);

    // We need to find the last EOCDH as there's a possibility that an inner ZIP file exists with the Stored
    // compression method, so matching that would be undesirable. For the moment, we match all EOCDHs
    // sequentially and store the latest's offest.
    // TODO: Seeking in reverse may be a better a approach for this - needs some testing.
    'outer: loop {
        'inner: loop {
            let mut buffer = [0; async_io_utilities::SUGGESTED_BUFFER_SIZE];
            if reader.read(&mut buffer).await? == 0 {
                break 'inner;
            }
        }

        if reader.matched() {
            let inner_offset = reader.get_mut().seek(SeekFrom::Current(0)).await?;
            matched_offset = Some(inner_offset - reader.buffer().len() as u64);
        } else if matched_offset.is_some() {
            break 'outer;
        } else {
            return Err(ZipError::UnexpectedHeaderError(0, crate::spec::signature::END_OF_CENTRAL_DIRECTORY));
        }

        reader.reset();
    }

    let mut reader = reader.into_inner();
    reader.seek(SeekFrom::Start(matched_offset.unwrap())).await?;
    let eocdh = EndOfCentralDirectoryHeader::from_reader(&mut reader).await?;

    // Outdated feature so unlikely to ever make it into this crate.
    if eocdh.disk_num != eocdh.start_cent_dir_disk || eocdh.num_of_entries != eocdh.num_of_entries_disk {
        return Err(ZipError::FeatureNotSupported("Spanned/split files"));
    }

    if eocdh.file_comm_length > 0 {
        comment = Some(async_io_utilities::read_string(&mut reader, eocdh.file_comm_length as usize).await?);
    }

    reader.seek(SeekFrom::Start(eocdh.cent_dir_offset.into())).await?;
    let mut entries = Vec::with_capacity(eocdh.num_of_entries.into());

    for _ in 0..eocdh.num_of_entries {
        entries.push(read_cd_entry(reader).await?);
    }

    Ok((entries, comment))
}

pub(crate) async fn read_cd_entry<R: AsyncRead + Unpin>(reader: &mut R) -> Result<ZipEntry> {
    crate::utils::assert_signature(reader, crate::spec::signature::CENTRAL_DIRECTORY_FILE_HEADER).await?;

    let header = CentralDirectoryHeader::from_reader(reader).await?;
    let filename = async_io_utilities::read_string(reader, header.file_name_length.into()).await?;
    let extra = async_io_utilities::read_bytes(reader, header.extra_field_length.into()).await?;
    let comment = async_io_utilities::read_string(reader, header.file_comment_length.into()).await?;

    let entry = ZipEntry {
        name: filename,
        comment: Some(comment),
        data_descriptor: header.flags.data_descriptor,
        crc32: Some(header.crc),
        uncompressed_size: Some(header.uncompressed_size),
        compressed_size: Some(header.compressed_size),
        last_modified: crate::spec::date::zip_date_to_chrono(header.mod_date, header.mod_time),
        extra: Some(extra),
        compression: Compression::from_u16(header.compression)?,
        offset: Some(header.lh_offset),
    };

    Ok(entry)
}
