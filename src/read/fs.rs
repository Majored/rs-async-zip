// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A concurrent ZIP reader which acts over a file system path.
//!
//! Concurrency is achieved as a result of:
//! - Wrapping the provided path within an [`Arc`] to allow shared ownership.
//! - Constructing a new [`File`] from the path when reading.
//!
//! ### Usage
//! Unlike the [`seek`] module, we no longer hold a mutable reference to any inner reader which in turn, allows the
//! construction of concurrent [`ZipEntryReader`]s. Though, note that each individual [`ZipEntryReader`] cannot be sent
//! between thread boundaries due to the masked lifetime requirement. Therefore, the overarching [`ZipFileReader`]
//! should be cloned and moved into those contexts when needed.
//!
//! ### Concurrent Example
//! ```no_run
//! # use async_zip::read::fs::ZipFileReader;
//! # use async_zip::error::Result;
//! # use tokio::io::AsyncReadExt;
//! #
//! async fn run() -> Result<()> {
//!     let reader = ZipFileReader::new("./foo.zip").await?;
//!     let result = tokio::join!(read(&reader, 0), read(&reader, 1));
//!
//!     let data_0 = result.0?;
//!     let data_1 = result.1?;
//!
//!     // Use data within current scope.
//!
//!     Ok(())
//! }
//!
//! async fn read(reader: &ZipFileReader, index: usize) -> Result<Vec<u8>> {
//!     let mut entry = reader.entry(index).await?;
//!     let mut data = Vec::new();
//!     entry.read_to_end(&mut data).await?;
//!     Ok(data)
//! }
//! ```
//!
//! ### Parallel Example
//! ```no_run
//! # use async_zip::read::fs::ZipFileReader;
//! # use async_zip::error::Result;
//! # use tokio::io::AsyncReadExt;
//! #
//! async fn run() -> Result<()> {
//!     let reader = ZipFileReader::new("./foo.zip").await?;
//!     
//!     let handle_0 = tokio::spawn(read(reader.clone(), 0));
//!     let handle_1 = tokio::spawn(read(reader.clone(), 1));
//!
//!     let data_0 = handle_0.await.expect("thread panicked")?;
//!     let data_1 = handle_1.await.expect("thread panicked")?;
//!
//!     // Use data within current scope.
//!
//!     Ok(())
//! }
//!
//! async fn read(reader: ZipFileReader, index: usize) -> Result<Vec<u8>> {
//!     let mut entry = reader.entry(index).await?;
//!     let mut data = Vec::new();
//!     entry.read_to_end(&mut data).await?;
//!     Ok(data)
//! }
//! ```

#[cfg(doc)]
use crate::read::seek;

use crate::error::{Result, ZipError};
use crate::file::ZipFile;
use crate::read::io::entry::ZipEntryReader;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::fs::File;
use tokio::io::BufReader;

struct Inner {
    path: PathBuf,
    file: ZipFile,
}

/// A concurrent ZIP reader which acts over a file system path.
#[derive(Clone)]
pub struct ZipFileReader {
    inner: Arc<Inner>,
}

impl ZipFileReader {
    /// Constructs a new ZIP reader from a file system path.
    pub async fn new<P>(path: P) -> Result<ZipFileReader>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref().to_owned();
        let file = crate::read::file(File::open(&path).await?).await?;

        Ok(ZipFileReader { inner: Arc::new(Inner { path, file }) })
    }

    /// Returns this ZIP file's information.
    pub fn file(&self) -> &ZipFile {
        &self.inner.file
    }

    /// Returns the file system path provided to the reader during construction.
    pub fn path(&self) -> &Path {
        &self.inner.path
    }

    /// Returns a new entry reader if the provided index is valid.
    pub async fn entry(&self, index: usize) -> Result<ZipEntryReader<File>> {
        let stored_entry = self.inner.file.entries.get(index).ok_or(ZipError::EntryIndexOutOfBounds)?;
        let mut fs_file = BufReader::new(File::open(&self.inner.path).await?);

        stored_entry.seek_to_data_offset(&mut fs_file).await?;

        Ok(ZipEntryReader::new_with_owned(
            fs_file,
            stored_entry.entry.compression(),
            stored_entry.entry.compressed_size(),
        ))
    }
}
