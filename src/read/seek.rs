// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A ZIP reader which acts over a seekable source.
//! 
//! ### Example
//! ```no_run
//! # use async_zip::read::seek::ZipFileReader;
//! # use async_zip::error::Result;
//! #
//! # async fn run() -> Result<()> {
//! let data: File::open("./foo.zip").await?;
//! let reader = ZipFileReader::new(data).await?;
//! 
//! let mut data = Vec::new();
//! let entry = reader.entry(0).await?;
//! entry.read_to_end(&mut data).await?;
//! 
//! #   Ok(())
//! # }
//! ```

use crate::error::{Result, ZipError};
use crate::file::ZipFile;
use crate::read::io::entry::ZipEntryReader;

use tokio::io::{AsyncRead, AsyncSeek, AsyncSeekExt, SeekFrom};

/// A ZIP reader which acts over a seekable source.
pub struct ZipFileReader<R> where R: AsyncRead + AsyncSeek + Unpin {
    reader: R,
    file: ZipFile,
}

impl<R> ZipFileReader<R> where R: AsyncRead + AsyncSeek + Unpin {
    /// Constructs a new ZIP reader from a seekable source.
    pub async fn new(mut reader: R) -> Result<ZipFileReader<R>> {
        let file = crate::read::file(&mut reader).await?;
        Ok(ZipFileReader { reader, file })
    }
    
    /// Returns this ZIP file's information.
    pub fn file(&self) -> &ZipFile {
        &self.file
    }

    /// Returns a new entry reader if the provided index is valid.
    pub async fn entry(&mut self, index: usize) -> Result<ZipEntryReader<'_, R>> {
        let entry = self.file.entries.get(index).ok_or(ZipError::EntryIndexOutOfBounds)?;
        let meta = self.file.metas.get(index).ok_or(ZipError::EntryIndexOutOfBounds)?;
        let seek_to = crate::read::compute_data_offset(&entry, &meta);

        self.reader.seek(SeekFrom::Start(seek_to)).await?;
        Ok(ZipEntryReader::new_with_borrow(&mut self.reader, entry.compression(), entry.uncompressed_size().into()))
    }
}