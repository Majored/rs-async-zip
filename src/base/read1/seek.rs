// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A ZIP archive reader which acts over a seekable source.
//! 
//! # Overview
//! A seeking reader provides many advantages over a streaming reader. See the parent [`super`] module for a comparison of the two.
//! 
//! # Opening an archive
//! ```no_run
//! # use async_zip::base::read1::seek::ZipArchiveReader;
//! # use futures_lite::io::Cursor;
//! # 
//! # async fn main() {
//! // With default options
//! let data = Cursor::new(Vec::new()); // Replace with your ZIP archive data
//! let reader = ZipArchiveReader::open(data).await.expect("failed to open zip archive");
//! 
//! // Or with custom options; 
//! let options = ZipOptionsBuilder::new().load_file_meta(false).into();
//! let data = Cursor::new(Vec::new()); // Replace with your ZIP archive data
//! let reader = ZipArchiveReader::open(data, options).await.expect("failed to open zip archive");
//! 
//! // Or with a known inner state (e.g. from a previous reader);
//! let inner = reader.inner().clone();
//! let reader = ZipArchiveReader::new_with_inner(data, inner);
//! 
//! // Or using a factory to produce new readers over the same archive concurrently/in parallel.
//! let data: Arc<[u8]> = Arc::from(Vec::new()); // Replace with your ZIP archive data
//! let generator = async move || -> Result<_> { Ok(Cursor::new(Arc::clone(&data))) };
//! let primary = generator().await.expect("failed to generate reader");
//! let factory = ZipArchiveReader::open(primary).await.expect("failed to open zip archive").into_factory(generator);
//! let reader = factory.reader().await.unwrap(); // Can be called multiple times
//! # }
//! ```
//! 
//! # Accessing archive metadata
//! ```no_run
//! # use async_zip::base::read1::seek::ZipArchiveReader;
//! # use futures_lite::io::Cursor;
//! # 
//! # async fn main() {
//! let data = Cursor::new(Vec::new()); // Replace with your ZIP archive data
//! let reader = ZipArchiveReader::open(data).await.expect("failed to open zip archive");
//! 
//! // Enumerate through the files in the archive
//! for (i, meta) in reader.metas().iter().enumerate() {
//!    println!("File {i}: {:?}", meta.file_name);
//! }
//! 
//! // Or find a file by its filename;
//! let index = reader.find(b"hello.txt").next().expect("failed to look up filename");
//! ```
//! 
//! # Opening a file for reading
//! ```no_run
//! # use async_zip::base::read1::seek::ZipArchiveReader;
//! # use futures_lite::io::Cursor;
//! # 
//! # async fn main() {
//! let data = Cursor::new(Vec::new()); // Replace with your ZIP archive data
//! let reader = ZipArchiveReader::open(data).await.expect("failed to open zip archive");
//! 
//! // Read files sequentially by index without consuming the source reader
//! let file = reader.file(0).await.expect("failed to open file at index 0");
//! let content = file.read_to_string().await.expect("failed to read file contents");
//! 
//! // Or start reading a file in oneshot fashion, consuming the source reader in the process
//! let file = reader.file_oneshot(0).await.expect("failed to open file at index 0");
//! let content = file.read_to_string().await.expect("failed to read file contents");
//! # }
//! ```

#[expect(unused_imports)]
use crate::error::ZipError;

use std::{io::SeekFrom, ops::Deref, sync::Arc};
use futures_lite::{AsyncBufRead, AsyncSeek, AsyncSeekExt};

use crate::{base::read1::{file::ZipFileReader, ops::{Ops, SeekOps}, opts::ZipOptions}, error::Result, spec::constructs::{CDR, LF}};

/// A ZIP archive reader which acts over a seekable source.
pub struct ZipArchiveReader<R> {
    inner: Arc<ZipArchiveInner>,
    reader: R,
}

impl<R: AsyncBufRead + AsyncSeek + Unpin> ZipArchiveReader<R> {
    /// Opens a ZIP archive with the default options.
    /// 
    /// # Errors
    /// [`ZipError::UnableToLocateEOCDR`]  
    /// [`ZipError::UnexpectedHeaderError`]  
    /// [`ZipError::UpstreamReadError`]  
    pub async fn open(mut reader: R) -> Result<Self> {
        let opts = ZipOptions::default();
        let inner = SeekOps::new(&mut reader).open(opts).await?;
        Ok(Self::new_with_inner(reader, Arc::new(inner)))
    }

    /// Opens a ZIP archive with the given options. See [`Self::open()`] for more information.
    pub async fn open_with_options(mut reader: R, opts: ZipOptions) -> Result<Self> {
        let inner = SeekOps::new(&mut reader).open(opts).await?;
        Ok(Self::new_with_inner(reader, Arc::new(inner)))
    }

    /// Constructs a new reader with the inner state of a known ZIP archive.
    /// 
    /// This is useful when opening the same archive multiple times, without re-reading
    /// the central directory each time. Providing a ZipArchiveInner which does not match
    /// the archive backing the provided reader will result in incorrect/unexpected parsing.
    pub fn new_with_inner(reader: R, inner: Arc<ZipArchiveInner>) -> Self {
        Self { reader, inner }
    }

    /// Returns the Arc to the inner state of the ZIP archive. Or use the Deref implementation.
    pub fn inner(&self) -> &Arc<ZipArchiveInner> {
        &self.inner
    }

    /// Opens a file for reading by its index in the archive.
    /// 
    /// # Errors
    /// [`ZipError::UnexpectedHeaderError`]  
    /// [`ZipError::UpstreamReadError`]. 
    pub async fn file(&mut self, index: usize) -> Result<ZipFileReader<&mut R>> {
        let lf = self.file_open(index).await?;
        ZipFileReader::new(&mut self.reader, lf)
    }

    /// Opens a file for reading by its index in the archive. See [`Self::file()`] for more information.
    /// 
    /// This takes an owned Self and consumes the source reader.
    pub async fn file_oneshot(mut self, index: usize) -> Result<ZipFileReader<R>> {
        let lf = self.file_open(index).await?;
        ZipFileReader::new(self.reader, lf)
    }

    /// Converts this reader into a [`ZipArchiveFactory`] which can produce new readers over the same archive.
    /// 
    /// The generator function must produce readers over the same archive this reader was opened with.
    pub fn into_factory<G: AsyncFn() -> Result<R>>(self, generator: G) -> ZipArchiveFactory<G, R> {
        ZipArchiveFactory::new(generator, self)
    }

    // HELPERS (not part of public API)

    async fn cdr(&mut self, index: usize) -> Result<CDR> {
        // Cloning the CDR is going to be cheaper than seeking back and re-reading.
        if let Some(cdr) = self.inner.cdr_metas.get(index) {
            return Ok(cdr.clone());
        }

        // Seek to the CDR offset.
        let offset = self.inner.cdr_offsets.get(index).unwrap(); // TODO
        self.reader.seek(SeekFrom::Start(*offset)).await?;

        // Read the CDR.
        Ops::new(&mut self.reader).cdr().await
    }

    async fn file_open(&mut self, index: usize) -> Result<LF> {
        // Read the CDR first (or fetch from memory if already loaded).
        let cdr = self.cdr(index).await?;

        // Read the LF and do any required validation.
        SeekOps::new(&mut self.reader).file(cdr, &self.inner.options).await
    }
}

impl<R> Deref for ZipArchiveReader<R> {
    type Target = ZipArchiveInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// An inner store of metadata about a ZIP archive, which can be shared between readers.
#[derive(Default, Clone)]
pub struct ZipArchiveInner {
    pub(crate) cdr_offsets: Vec<u64>,
    pub(crate) cdr_metas: Vec<CDR>,
    pub(crate) options: ZipOptions,
}

impl ZipArchiveInner {
    /// Finds all the file indexes with the given filename. An iterator is returned because the ZIP specification
    /// allows for multiple files with the same filename, and most callers only need the first match.
    pub fn find<'a>(&'a self, filename: &'a [u8]) -> impl Iterator<Item = usize> + 'a {
        self.cdr_metas.iter().enumerate().filter_map(move |(i, cdr)| (cdr.file_name == filename).then_some(i))
    }

    pub fn metas(&self) -> &[CDR] {
        &self.cdr_metas
    }

    pub fn options(&self) -> &ZipOptions {
        &self.options
    }
}

/// Allows for concurrent/parallel reads of the same archive using a generator function.
/// 
/// Note that internally this stores the ZipArchiveReader which was used to construct it, meaning at least one reader
/// is open at all times.
pub struct ZipArchiveFactory<G, R> {
    primary: ZipArchiveReader<R>,
    generator: G,
}

impl <R: AsyncBufRead + AsyncSeek + Unpin, G: AsyncFn() -> Result<R>> ZipArchiveFactory<G, R> {
    /// Wraps an open [`ZipArchiveReader`] with a generator function which produces new readers.
    /// 
    /// The generator function must produce readers over the same archive this reader was opened with.
    pub fn new(generator: G, primary: ZipArchiveReader<R>) -> Self{
        Self { generator, primary }
    }

    pub fn primary(&self) -> &ZipArchiveReader<R> {
        &self.primary
    }

    /// 
    pub async fn reader(&self) -> Result<ZipArchiveReader<R>> {
        let reader = (self.generator)().await?;
        let inner = self.primary.inner().clone();
        Ok(ZipArchiveReader { reader, inner })
    }
}
