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
use crate::read::{OwnedReader, PrependReader, ZipEntry, ZipEntryReader};
use crate::spec::header::LocalFileHeader;
use crate::read::ZipEntryMeta;

use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncSeekExt;

/// A reader which acts concurrently over a filesystem file.
pub struct ZipFileReader {
    pub(crate) filename: PathBuf,
    pub(crate) entries: Vec<(ZipEntry, ZipEntryMeta)>,
    pub(crate) comment: Option<String>,
}

impl ZipFileReader {
    /// Constructs a new ZIP file reader from a filename.
    pub async fn new<P: AsRef<Path>>(filename: P) -> Result<ZipFileReader> {
        let mut fs_file = File::open(&filename).await?;
        let (entries, comment) = crate::read::seek::read_cd(&mut fs_file).await?;

        Ok(ZipFileReader { filename: filename.as_ref().to_path_buf(), entries, comment })
    }

    crate::read::reader_entry_impl!();

    /// Opens an entry at the provided index for reading.
    pub async fn entry_reader(&self, index: usize) -> Result<ZipEntryReader<'_, File>> {
        let entry = self.entries.get(index).ok_or(ZipError::EntryIndexOutOfBounds)?;

        let mut fs_file = File::open(&self.filename).await?;
        fs_file.seek(SeekFrom::Start(entry.1.file_offset.unwrap() as u64 + 4)).await?;

        let header = LocalFileHeader::from_reader(&mut fs_file).await?;
        let data_offset = (header.file_name_length + header.extra_field_length) as i64;
        fs_file.seek(SeekFrom::Current(data_offset)).await?;

        let reader = OwnedReader::Owned(fs_file);
        let reader = PrependReader::Normal(reader);
        let reader = CompressionReader::from_reader(&entry.0.compression(), reader, Some(entry.0.compressed_size()).map(u32::into))?;

        Ok(ZipEntryReader::from_raw(&entry.0, &entry.1, reader, entry.1.general_purpose_flag.data_descriptor))
    }
}
