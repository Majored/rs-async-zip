// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use std::{io::SeekFrom};
use futures_lite::{AsyncRead, AsyncSeek, AsyncSeekExt};

use crate::{base::read1::{file::{self, ZipFileReader}, ops::{Ops, SeekOps}, opts::Options}, error::Result, spec::headers1::{CDR, CDRH}};

pub struct ZipArchiveReader<R> {
    inner: SeekInner,
    reader: R,
}

impl<R: AsyncRead + AsyncSeek + Unpin> ZipArchiveReader<R> {
    pub async fn open(mut reader: R) -> Result<Self> {
        let inner = SeekOps::new(&mut reader).open().await?;
        Ok(Self { reader, inner })
    }

    pub async fn open_with_options(reader: R, options: Options) -> Result<Self> {
        let inner = SeekInner::with_options(options);
        Ok(Self { reader, inner })
    }

    /// Finds all the file indexes with the given filename.
    /// 
    /// A vector is returned because the ZIP specification allows for multiple files with the same filename.
    pub fn find(&self, filename: &[u8]) -> Result<Vec<usize>> {
        self.inner.find(filename)
    }

    /// Returns
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

    /// Opens a file for reading by its index in the archive.
    pub async fn file(&mut self, index: usize) -> Result<ZipFileReader<&mut R>> {
        // Read the CDR first (or fetch from memory if already loaded).
        let cdr = self.cdr(index).await?;

        // Read the LF and do any required validation.
        let lf = SeekOps::new(&mut self.reader).file(cdr, &self.inner.options).await?;

        // We've read the LF and are now at the start of the data.
        ZipFileReader::new(&mut self.reader, lf)
    }
}

#[derive(Default)]
pub(crate) struct SeekInner {
    pub cdr_offsets: Vec<u64>,
    pub cdr_metas: Vec<CDR>,
    pub options: Options,
}

impl SeekInner {
    pub fn with_options(options: Options) -> Self {
        Self {
            cdr_offsets: Vec::new(),
            cdr_metas: Vec::new(),
            options,
        }
    }

    // TODO: allow option to store lfh instead, so linear reads don't throw away buffer.

    /// Returns the indexes of all files with the given filename.
    /// 
    /// A vector is returned because the ZIP specification allows for multiple files with the same filename.
    pub fn find(&self, filename: &[u8]) -> Result<Vec<usize>> {
        if !self.options.load_file_meta {
            return Err(crate::error::ZipError::FileMetaNotLoaded);
        }

        Ok(self.cdr_metas.iter().enumerate().filter_map(|(i, cdr)| {
            if cdr.file_name == filename {
                Some(i)
            } else {
                None
            }
        }).collect())
    }
}
