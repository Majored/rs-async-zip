// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A ZIP reader which acts over a seekable source.
//!
//! ### Example
//! ```no_run
//! # use async_zip::read::seek::ZipFileReader;
//! # use async_zip::error::Result;
//! # use tokio::io::AsyncReadExt;
//! # use tokio::fs::File;
//! #
//! async fn run() -> Result<()> {
//!     let mut data = File::open("./foo.zip").await?;
//!     let mut reader = ZipFileReader::new(&mut data).await?;
//! 
//!     let mut data = Vec::new();
//!     let mut entry = reader.entry(0).await?;
//!     entry.read_to_end(&mut data).await?;
//! 
//!     // Use data within current scope.
//! 
//!     Ok(())
//! }
//! ```

use crate::error::{Result, ZipError};
use crate::file::ZipFile;
use crate::read::io::entry::ZipEntryReader;

use tokio::io::{AsyncRead, AsyncSeek, AsyncSeekExt, SeekFrom};

/// A ZIP reader which acts over a seekable source.
pub struct ZipFileReader<R> {
    reader: R,
    file: ZipFile,
}

impl<R> ZipFileReader<R>
where
    R: AsyncRead + AsyncSeek + Unpin,
{
    /// Constructs a new ZIP reader from a seekable source.
    pub async fn new(mut reader: R) -> Result<ZipFileReader<R>> {
        let file = crate::read::file(&mut reader).await?;
        Ok(ZipFileReader { reader, file })
    }

    /// Returns this ZIP file's information.
    pub fn file(&self) -> &ZipFile {
        &self.file
    }

    /// Returns a mutable reference to the inner reader
    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    /// Unwraps this `ZipFileReader<R>`, returning the underlying reader.
    pub fn into_inner(self) -> R {
        self.reader
    }

    /// Returns a new entry reader if the provided index is valid.
    pub async fn entry(&mut self, index: usize) -> Result<ZipEntryReader<'_, R>> {
        let stored_entry = self.file.entries.get(index).ok_or(ZipError::EntryIndexOutOfBounds)?;
        let seek_to = stored_entry.data_offset();

        self.reader.seek(SeekFrom::Start(seek_to)).await?;
        Ok(ZipEntryReader::new_with_borrow(
            &mut self.reader,
            stored_entry.entry.compression(),
            stored_entry.entry.uncompressed_size().into(),
        ))
    }
}
