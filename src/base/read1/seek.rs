// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A ZIP archive reader which acts over a seekable source.
//! 
//! # Overview
//! A seeking reader provides many advantages over a streaming reader. See the parent [`super`] module for a comparison of the two.
//! 
//! # Using a factory
//! [`ZipArchiveFactory`] allows you to open concurrent/parallel [`ZipArchiveReader`]s over the same archive.
//! 
//! Instead of providing a reader, you provide a factory/generator function which produces new readers.
//! For example, a filesystem file, or a cursor over a file stored in an in-memory buffer.
//! Combine this with a Semaphore to limit the number of concurrent readers.
//! 
//! ```no_run
//! let path = "file.zip";
//! let data: Arc<[u8]> = Arc::from(tokio::fs::read(path).await?);
//! let generator = async move || -> Result<_> { Ok(Cursor::new(Arc::clone(&data))) };
//! let factory = ZipArchiveReader::open(generator().await?).await?.into_factory(generator);
//!
//! for (i, meta) in factory.primary().inner().metas().iter().enumerate() {
//!     let reader = factory.reader().await.unwrap();
//!
//!     tokio::spawn(async move {
//!         let res = reader.find(b"hello.txt").unwrap();
//!         println!("Found indexes: {:?}", res);
//!     }).await.unwrap();
//! }
//! ```
//! # Opening an archive
//! 
//! // with options
//! 
//! 

use std::{io::SeekFrom, sync::Arc};
use futures_lite::{AsyncBufRead, AsyncSeek, AsyncSeekExt};

use crate::{base::read1::{file::ZipFileReader, ops::{Ops, SeekOps}, opts::Options}, error::Result, spec::headers1::{CDR, LF}};

/// A ZIP archive reader which acts over a seekable source.
pub struct ZipArchiveReader<R> {
    inner: Arc<ZipArchiveInner>,
    reader: R,
}

impl<R: AsyncBufRead + AsyncSeek + Unpin> ZipArchiveReader<R> {
    /// Opens a ZIP archive with the default options.
    /// 
    /// # Errors
    /// EndOfCentralDirectoryRecordNotFound - if the EOCDR cannot be found in the archive.
    /// UnexpectedHeaderError - if the EOCDR signature is not found at the expected location.
    /// UpstreamReadError - if the underlying reader returns an error while reading.
    pub async fn open(mut reader: R) -> Result<Self> {
        let opts = Options::default();
        let inner = SeekOps::new(&mut reader).open(opts).await?;
        Ok(Self::new_with_inner(reader, Arc::new(inner)))
    }

    /// Opens a ZIP archive with the given options. See `open()` for more information.
    pub async fn open_with_options(mut reader: R, opts: Options) -> Result<Self> {
        let inner = SeekOps::new(&mut reader).open(opts).await?;
        Ok(Self::new_with_inner(reader, Arc::new(inner)))
    }

    /// Constructs a new reader with the inner state of a known ZIP archive.
    /// 
    /// This is useful when opening the same archive multiple times, without re-reading
    /// the central directory each time. 
    pub fn new_with_inner(reader: R, inner: Arc<ZipArchiveInner>) -> Self {
        Self { reader, inner }
    }

    /// Finds all the file indexes with the given filename.
    /// 
    /// A vector is returned because the ZIP specification allows for multiple files with the same filename.
    pub fn find(&self, filename: &[u8]) -> Result<Vec<usize>> {
        // TODO: zero-allocation approach?

        if !self.inner.options.load_file_meta {
            return Err(crate::error::ZipError::FileMetaNotLoaded);
        }

        Ok(self.inner.cdr_metas.iter().enumerate().filter_map(|(i, cdr)| {
            if cdr.file_name == filename {
                Some(i)
            } else {
                None
            }
        }).collect())
    }

    /// Validates that the file at the given index can be opened.
    pub async fn validate(&mut self, index: usize) -> Result<()> {
        self.file_open(index).await?;
        Ok(())
    }

    /// Returns a reference to the inner state of the ZIP archive.
    pub fn inner(&self) -> &Arc<ZipArchiveInner> {
        &self.inner
    }

    /// Opens a file for reading by its index in the archive.
    pub async fn file(&mut self, index: usize) -> Result<ZipFileReader<&mut R>> {
        let lf = self.file_open(index).await?;
        ZipFileReader::new(&mut self.reader, lf)
    }

    /// Opens a file for reading by its index in the archive, consuming the archive reader.
    pub async fn file_oneshot(mut self, index: usize) -> Result<ZipFileReader<R>> {
        let lf = self.file_open(index).await?;
        ZipFileReader::new(self.reader, lf)
    }

    /// Converts this reader into a [`ZipArchiveFactory`] which can produce new readers over the same archive.
    /// 
    /// The generator function must produce readers over the same archive this reader was opened with.
    pub async fn into_factory<G: AsyncFn() -> Result<R>>(self, generator: G) -> Result<ZipArchiveFactory<G, R>> {
        Ok(ZipArchiveFactory::new(generator, self))
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

/// An inner store of metadata about a ZIP archive, which can be shared between readers.
#[derive(Default, Clone)]
pub struct ZipArchiveInner {
    pub(crate) cdr_offsets: Vec<u64>,
    pub(crate) cdr_metas: Vec<CDR>,
    pub(crate) options: Options,
}

impl ZipArchiveInner {
    pub fn with_options(options: Options) -> Self {
        Self {
            cdr_offsets: Vec::new(),
            cdr_metas: Vec::new(),
            options,
        }
    }

    pub fn metas(&self) -> &[CDR] {
        &self.cdr_metas
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
