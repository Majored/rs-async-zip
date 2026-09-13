// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A ZIP archive streaming reader which acts over a non-seekable source.
//! 
//! # Overview
//! A seeking reader provides many advantages over a streaming reader. See the parent [`super`] module for a comparison of the two.
//! 
//! # Opening an archive
//! ```no_run
//! # use async_zip::base::read1::stream::ZipArchiveReader;
//! # use async_zip::base::read1::ZipOptions;
//! # use async_zip::error::Result;
//! # use futures_lite::io::Cursor;
//! # use std::sync::Arc;
//! # 
//! # async fn main2() {
//! // With default options
//! let data = Cursor::new(Vec::new()); // Replace with your ZIP archive data
//! let archive = ZipArchiveReader::new(data);
//! 
//! // Or with custom options; 
//! let options = ZipOptions { max_cd_num_files: 16, ..Default::default() };
//! let data = Cursor::new(Vec::new()); // Replace with your ZIP archive data
//! let archive = ZipArchiveReader::new_with_options(data, options);
//! # }
//! ```
//! 
//! # Reading local files
//! ```no_run
//! # use async_zip::base::read1::stream::ZipArchiveReader;
//! # use futures_lite::io::{Cursor, AsyncReadExt};
//! # 
//! # async fn main2() {
//! let data = Cursor::new(Vec::new()); // Replace with your ZIP archive data
//! let mut archive = ZipArchiveReader::new(data);
//! 
//! while let Some(file) = archive.next().await.expect("failed to read next file") {
//!     let mut content = String::new();
//!     file.read_to_string(&mut content).await.expect("failed to read file contents");
//!     // EOF reached so size & CRC has been validated.
//! }
//! 
//! // As next() returned None, the central directory has been fully read and validated.
//! # }
//! ```
//! 
//! # Accessing archive metadata
//! ```no_run
//! # use async_zip::base::read1::stream::ZipArchiveReader;
//! # use futures_lite::io::{Cursor, AsyncReadExt};
//! # 
//! # async fn main2() {
//! let data = Cursor::new(Vec::new()); // Replace with your ZIP archive data
//! let mut archive = ZipArchiveReader::new(data);
//! 
//! // Drop all the local file readers without doing any work.
//! while let Some(file) = archive.next().await.expect("failed to read next file") {}
//! // At this point, cdrs() and ceocdr() are Some, and lfs() has been filled.
//! 
//! for (i, cdr) in archive.cdrs().expect("has cdrs").iter().enumerate() {
//!    println!("File {i}: {:?}", cdr.insecure_file_name);
//! }
//! # }
//! ```

// TODO: docs about raw header streams

use crate::error::ZipError;
use crate::spec::constructs::{CDR, CEOCDR, EOCDR};
use crate::spec::headers1::{Compression, EOCDL64H, EOCDR64H, Signature};

use futures_lite::AsyncBufRead;

use crate::{base::read1::{file::ZipFileReader, ops::{Ops}, opts::ZipOptions}, error::Result, spec::constructs::{LF}};

/// A ZIP archive streaming reader which acts over a non-seekable source.
pub struct ZipArchiveReader<R> {
    state: Option<State<R>>,
    options: ZipOptions,
    lfs: Vec<LF>,
    cdrs: Option<Vec<CDR>>,
    ceocdr: Option<CEOCDR>,
    next_header: Option<ZipStreamHeader>,
}

impl<R: AsyncBufRead + Unpin> ZipArchiveReader<R> {
    /// Opens a ZIP archive with the default options.
    pub fn new(reader: R) -> Self {
        Self::new_with_options(reader, ZipOptions::default())
    }

    /// Opens a ZIP archive with the given options. See [`Self::new()`] for more information.
    pub fn new_with_options(reader: R, options: ZipOptions) -> Self {
        Self { state: Some(State::Ready(reader)), options, lfs: Vec::new(), cdrs: None, ceocdr: None, next_header: None }
    }

    /// Returns a reader for the next file in the ZIP archive.
    /// If you get an error, you may continue to call this function to try to read the next file.
    /// 
    /// Once None is returned:
    /// - there are no more files in the archive
    /// - the end of the archive has been fully read which includes:
    ///     - all central directory records
    ///     - the ZIP64 end of central directory record, if it exists
    ///     - the ZIP64 end of central directory locator, if it exists
    ///     - the end of central directory record
    ///     - the archive comment
    /// - all configured validations have been performed
    pub async fn next(&mut self) -> Result<Option<&mut ZipFileReader<R>>> {
        let next = self.raw_next_header().await?;

        if let ZipStreamHeader::LF(lf) = next {
            return Ok(Some(self.raw_assume_lf(lf).await?));
        }

        self.next_header = Some(next);
        self.consume_eoa().await?;
        
        Ok(None)
    }

    /// Returns the list of local files which have been read so far.
    /// 
    /// You must continue to call [`Self::next()`] for this list to be populated.
    pub fn lfs(&self) -> &[LF] {
        &self.lfs
    }

    /// Returns the list of central directory records in the ZIP archive.
    /// 
    /// This is only available after [`Self::next()`] has returned `None` and the central directory has been validated.
    pub fn cdrs(&self) -> Option<&[CDR]> {
        self.cdrs.as_ref().map(|v| &**v)
    }

    /// Returns the combined end of central directory record for the ZIP archive.
    /// 
    /// This is only available after [`Self::next()`] has returned `None` and the central directory has been validated.
    pub fn ceocdr(&self) -> Option<&CEOCDR> {
        self.ceocdr.as_ref()
    }

    /// Returns the next raw header in the ZIP archive. Once [`ZipStreamHeader::EOCDR`] has been returned,
    /// no further headers will be available.
    /// 
    /// Calling this invalidates [`Self::next()`]'s state, so must be combined with [`Self::raw_assume_lf()`].
    pub async fn raw_next_header(&mut self) -> Result<ZipStreamHeader> {
        self.finish_current().await?;

        if let Some(next_header) = self.next_header.take() {
            return Ok(next_header);
        }

        let options = self.options.clone();
        let mut reader = self.mut_ready_reader();
        let signature = crate::spec::headers1::read::<Signature, R>(&mut reader).await?;

        // We shouldn't ever get to EOF before one of these signatures.
        match signature {
            Signature::LFH => {
                let lf = Ops::new(&mut reader, &options).lf(false).await?;
                return Ok(ZipStreamHeader::LF(lf));
            },
            Signature::CDRH => {
                let cdr = Ops::new(&mut reader, &options).cdr(false).await?;
                return Ok(ZipStreamHeader::CDR(cdr));
            },
            Signature::EOCDRH => {
                let eocdr = Ops::new(&mut reader, &options).eocdr().await?;
                return Ok(ZipStreamHeader::EOCDR(eocdr));
            },
            Signature::EOCDR64H => {
                let eocdr64h = crate::spec::headers1::read::<EOCDR64H, R>(reader).await?;
                return Ok(ZipStreamHeader::EOCDR64H(eocdr64h));
            },
            Signature::EOCDL64H => {
                let eocdl64h = crate::spec::headers1::read::<EOCDL64H, R>(reader).await?;
                return Ok(ZipStreamHeader::EOCDL64H(eocdl64h));
            },
            signature => {
                return Err(ZipError::UnexpectedHeaderError(signature.into(), Signature::LFH.into()));
            }
        }
    }

    /// Returns a file reader based on the assumption that the stream is positioned
    /// at the start of the local file specified by `lf`.
    /// 
    /// Paired with [`Self::raw_next_header()`].
    pub async fn raw_assume_lf(&mut self, lf: LF) -> Result<&mut ZipFileReader<R>> {
        self.finish_current().await?;

        let options = self.options.clone();
        let mut reader = self.mut_ready_reader();

        if lf.lfh.gpf.data_descriptor() {
            // TODO: add support back
            return Err(ZipError::FeatureNotSupported("stream reading data descriptors"));
        }
        if lf.lfh.gpf.data_descriptor() && lf.lfh.compression == Compression::Stored {
            return Err(ZipError::FeatureNotSupported("stream reading data descriptors with stored compression"));
        }

        // TODO: Needed to ensure that creating a ZipFileReader does not error before taking ownership of the reader.
        let _ = ZipFileReader::new(&mut reader, lf.clone(), None, options.clone())?;
        let reader = self.take_ready_reader();

        self.lfs.push(lf.clone());
        self.state = Some(State::Reading(ZipFileReader::new(reader, lf, None, options.clone())?));

        match &mut self.state {
            Some(State::Reading(entry)) => Ok(entry),
            _ => unreachable!(),
        }
    }

    /// Drains whatever is left of the current entry (if any) and reclaims the reader.
    async fn finish_current(&mut self) -> Result<()> {
        let state = self.state.take().expect("ZipArchiveReader state invariant violated");

        match state {
            State::Ready(reader) => {
                self.state = Some(State::Ready(reader));
                Ok(())
            },
            State::Reading(mut entry) => {
                let result = entry.skip().await;
                self.state = Some(State::Ready(entry.into_inner()));
                result
            }
        }
    }

    fn take_ready_reader(&mut self) -> R {
        match self.state.take().expect("ZipArchiveReader state invariant violated") {
            State::Ready(reader) => reader,
            State::Reading(_) => panic!("Cannot take ready reader while reading an entry"),
        }
    }

    fn mut_ready_reader(&mut self) -> &mut R {
        match self.state.as_mut().expect("ZipArchiveReader state invariant violated") {
            State::Ready(reader) => reader,
            State::Reading(_) => panic!("Cannot get mutable reference to ready reader while reading an entry"),
        }
    }

    async fn consume_eoa(&mut self) -> Result<()> {
        if !self.options.stream_fully_consume_archive {
            return Ok(());
        }

        let mut cdrs = Vec::with_capacity(self.lfs.len());
        let mut eocdr64h = None;
        let mut eocdl64h = None;

        loop {
            match self.raw_next_header().await? {
                ZipStreamHeader::LF(_) => return Err(ZipError::MalformedOutOfOrderHeader),
                ZipStreamHeader::CDR(cdr) => {
                    if eocdl64h.is_some() || eocdr64h.is_some() {
                        return Err(ZipError::MalformedOutOfOrderHeader);
                    }

                    cdrs.push(cdr);
                },
                ZipStreamHeader::EOCDR64H(actual_eocdr64h) => {
                    if eocdl64h.is_some() || eocdr64h.is_some() {
                        return Err(ZipError::MalformedOutOfOrderHeader);
                    }

                    eocdr64h = Some(actual_eocdr64h);
                },
                ZipStreamHeader::EOCDL64H(actual_eocdl64h) => {
                    if eocdl64h.is_some() || eocdr64h.is_none() {
                        return Err(ZipError::MalformedOutOfOrderHeader);
                    }

                    eocdl64h = Some(actual_eocdl64h);
                },
                ZipStreamHeader::EOCDR(eocdr) => {
                    self.ceocdr = Some(CEOCDR { eocdr, eocdr64h: eocdr64h, eocdl64h: eocdl64h });
                    self.cdrs = Some(cdrs);

                    if self.ceocdr.as_ref().unwrap().has_xor_headers() {
                        return Err(ZipError::MalformedMissingHeader);
                    }

                    break;
                },
            }
        }

        self.validate_eoa().await
    }

    async fn validate_eoa(&mut self,) -> Result<()> {
        let cdrs = self.cdrs().unwrap();
        let options = self.options.clone();

        if self.options.validate_cd_against_seen_when_streaming {
            for (i, cdr) in cdrs.iter().enumerate() {
                let lf = self.lfs().get(i).ok_or(ZipError::MoreLFHsThanCDRs)?;
                crate::base::read1::valid::validate_file(lf, cdr, &self.options)?;
            }
        }

        // TODO: can this be moved into eocdr() helper.
        Ops::new(self.mut_ready_reader(), &options).validate_eoa_is_eor().await?;
        crate::base::read1::valid::validate_archive(self.ceocdr().unwrap(), &options)?;

        // TODO: To validate lfh offsets, we'd need an AsyncOffsetReader. 

        Ok(())
    }
}

/// A set of possible headers that can be encountered in a ZIP stream.
pub enum ZipStreamHeader {
    LF(LF),
    CDR(CDR),
    EOCDR(EOCDR),
    EOCDR64H(EOCDR64H),
    EOCDL64H(EOCDL64H),
}

enum State<R> {
    Ready(R),
    Reading(ZipFileReader<R>),
}
