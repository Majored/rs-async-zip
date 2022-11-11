// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A concurrent ZIP reader which acts over an owned vector of bytes.
//! 
//! Concurrency is achieved as a result of:
//! - Wrapping the provided vector of bytes within an [`Arc`] to allow shared ownership.
//! - Wrapping this [`Arc`] around a [`Cursor`] when reading (as the [`Arc`] can deref and coerce into a `&[u8]`).
//! 
//! ### Usage
//! Unlike the [`seek`] module, we no longer hold a mutable reference to any inner reader which in turn, allows the
//! construction of concurrent [`ZipEntryReader`]s. Though, note that each individual [`ZipEntryReader`] cannot be sent
//! between thread boundaries due to the masked lifetime requirement. Therefore, the overarching [`ZipFileReader`]
//! should be cloned and moved into those contexts when needed.
//! 
//! ### Concurrent Example
//! ```no_run
//! # use async_zip::read::mem::ZipFileReader;
//! # use async_zip::error::Result;
//! #
//! # async fn run() -> Result<()> {
//! let data: Vec<u8> = Vec::new();
//! let reader = ZipFileReader::new(data).await?;
//! 
//! let fut_gen = |index| { async {
//!     let mut entry_reader = local_reader.entry_reader(index).await?;
//!     let mut data = Vec::new();
//!     entry_reader.read_to_end(&mut data).await?;
//! }};
//! 
//! tokio::join!(fut_gen(0), fut_gen(1)).map(|res| res?);
//! #   Ok(())
//! # }
//! ```
//! 
//! ### Parallel Example
//! ```no_run
//! # use async_zip::read::mem::ZipFileReader;
//! # use async_zip::error::Result;
//! #
//! # async fn run() -> Result<()> {
//! let data: Vec<u8> = Vec::new();
//! let reader = ZipFileReader::new(data).await?;
//! 
//! let fut_gen = |index| {
//!     let local_reader = reader.clone();
//! 
//!     tokio::spawn(async move {
//!         let mut entry_reader = local_reader.entry_reader(index).await?;
//!         let mut data = Vec::new();
//!         entry_reader.read_to_end(&mut data).await.unwrap();
//!     })
//! };
//! 
//! tokio::join!(fut_gen(0), fut_gen(1)).map(|res| res?);
//! #   Ok(())
//! # }
//! ```

#[cfg(doc)]
use crate::read::seek;

use crate::read::io::entry::ZipEntryReader;
use crate::file::ZipFile;
use crate::error::{Result, ZipError};

use std::sync::Arc;
use std::io::Cursor;

use tokio::io::{AsyncSeekExt, SeekFrom};

struct Inner {
    data: Vec<u8>,
    file: ZipFile,
}

// A concurrent ZIP reader which acts over an owned vector of bytes.
#[derive(Clone)]
pub struct ZipFileReader {
    inner: Arc<Inner>,
}

impl ZipFileReader {
    /// Constructs a new ZIP reader from an owned vector of bytes.
    pub async fn new(data: Vec<u8>) -> Result<ZipFileReader> {
        let file = crate::read::file(Cursor::new(&data)).await?;
        Ok(ZipFileReader { inner: Arc::new(Inner { data, file }) })
    }

    /// Returns this ZIP file's information.
    pub fn file(&self) -> &ZipFile {
        &self.inner.file
    }

    /// Returns the raw bytes provided to the reader during construction.
    pub fn data(&self) -> &[u8] {
        &self.inner.data
    }

    /// Returns a new entry reader if the provided index is valid.
    pub async fn entry(&self, index: usize) -> Result<ZipEntryReader<Cursor<&[u8]>>> {
        let entry = self.inner.file.entries.get(index).ok_or(ZipError::EntryIndexOutOfBounds)?;
        let meta = self.inner.file.metas.get(index).ok_or(ZipError::EntryIndexOutOfBounds)?;
        let seek_to = crate::read::compute_data_offset(&entry, &meta);
        let mut cursor = Cursor::new(&self.inner.data[..]);
        
        cursor.seek(SeekFrom::Start(seek_to)).await?;
        Ok(ZipEntryReader::new_with_owned(cursor, entry.compression(), entry.uncompressed_size().into()))
    }
}