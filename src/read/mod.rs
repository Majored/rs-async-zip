// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module which supports reading ZIP files using various approaches.

pub mod fs;
pub mod mem;
pub mod seek;
pub mod stream;
pub mod sync;

use crate::error::{Result, ZipError};
use crate::spec::compression::Compression;

use std::convert::TryInto;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_compression::tokio::bufread::{BzDecoder, DeflateDecoder, LzmaDecoder, XzDecoder, ZstdDecoder};
use chrono::{DateTime, Utc};
use crc32fast::Hasher;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, BufReader, ReadBuf, Take};
use async_io_utilities::AsyncDelimiterReader;

/// An entry within a larger ZIP file reader.
#[derive(Debug)]
pub struct ZipEntry {
    pub(crate) name: String,
    pub(crate) comment: Option<String>,
    pub(crate) data_descriptor: bool,
    pub(crate) crc32: Option<u32>,
    pub(crate) uncompressed_size: Option<u32>,
    pub(crate) compressed_size: Option<u32>,
    pub(crate) last_modified: DateTime<Utc>,
    pub(crate) extra: Option<Vec<u8>>,
    pub(crate) compression: Compression,

    // Additional fields from EOCDH.
    pub(crate) offset: Option<u32>,
}

impl ZipEntry {
    /// Returns a shared reference to the entry's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns an optional shared reference to the entry's comment.
    pub fn comment(&self) -> Option<&str> {
        match &self.comment {
            Some(comment) => Some(comment),
            None => None,
        }
    }

    /// Returns whether or not a data descriptor exists for the entry (ie. whether or not it was stream written).
    pub fn data_descriptor(&self) -> bool {
        self.data_descriptor
    }

    /// Returns whether or not the entry represents a directory.
    pub fn dir(&self) -> bool {
        self.name.ends_with("/")
    }

    /// Returns an optional CRC32 value for the entry.
    pub fn crc32(&self) -> Option<u32> {
        self.crc32
    }

    /// Returns an optional compressed file size for the entry.
    pub fn compressed_size(&self) -> Option<u32> {
        self.compressed_size
    }

    /// Returns an optional uncompressed file size for the entry.
    pub fn uncompressed_size(&self) -> Option<u32> {
        self.uncompressed_size
    }

    /// Returns a shared reference to the entry's last modification date.
    pub fn last_modified(&self) -> &DateTime<Utc> {
        &self.last_modified
    }

    /// Returns an optional shared reference to the extra bytes for the entry.
    pub fn extra(&self) -> Option<&Vec<u8>> {
        self.extra.as_ref()
    }

    /// Returns a shared reference to the compression type of the entry.
    pub fn compression(&self) -> &Compression {
        &self.compression
    }

    /// Returns the offset at which data for this entry starts.
    pub(crate) fn data_offset(&self) -> u64 {
        30 + self.offset.unwrap() as u64 + (self.name().len() + self.extra().unwrap().len()) as u64
    }
}

pub(crate) enum LocalReader<'a, R: AsyncRead + Unpin> {
    Standard(CompressionReader<'a, R>),
    Stream(CompressionReader<'a, AsyncDelimiterReader<&'a mut R>>),
}

impl<'a, R: AsyncRead + Unpin> AsyncRead for LocalReader<'a, R> {
    fn poll_read(mut self: Pin<&mut Self>, c: &mut Context<'_>, b: &mut ReadBuf<'_>) -> Poll<tokio::io::Result<()>> {
        match *self {
            LocalReader::Standard(ref mut inner) => Pin::new(inner).poll_read(c, b),
            LocalReader::Stream(ref mut inner) => Pin::new(inner).poll_read(c, b)
        }
    }
}

/// A ZIP file entry reader which may implement decompression.
pub struct ZipEntryReader<'a, R: AsyncRead + Unpin> {
    pub(crate) entry: &'a ZipEntry,
    pub(crate) reader: LocalReader<'a, R>,
    pub(crate) hasher: Hasher,
    pub(crate) consumed: bool,
}

impl<'a, R: AsyncRead + Unpin> ZipEntryReader<'a, R> {
    /// Construct an entry reader from its raw parts (a shared reference to the entry and an inner reader).
    pub(crate) fn from_raw(entry: &'a ZipEntry, reader: CompressionReader<'a, R>, _: bool) -> Self {
        let reader = LocalReader::Standard(reader);
        ZipEntryReader { entry, reader, hasher: Hasher::new(), consumed: false }
    }

    /// Construct an entry reader from its raw parts (a shared reference to the entry and an inner reader).
    pub(crate) fn with_data_descriptor(entry: &'a ZipEntry, reader: CompressionReader<'a, AsyncDelimiterReader<&'a mut R>>, _: bool) -> Self {
        let reader = LocalReader::Stream(reader);
        ZipEntryReader { entry, reader, hasher: Hasher::new(), consumed: false }
    }

    /// Returns a reference to the inner entry's data.
    pub fn entry(&self) -> &ZipEntry {
        self.entry
    }

    ///  Returns whether or not this reader has been fully consumed.
    pub fn consumed(&self) -> bool {
        self.consumed
    }

    /// Returns true if the computed CRC32 value of all bytes read so far matches the expected value.
    pub fn compare_crc(&mut self) -> bool {
        let hasher = std::mem::take(&mut self.hasher);
        self.entry.crc32().unwrap() == hasher.finalize()
    }

    /// A convenience method similar to `AsyncReadExt::read_to_end()` but with the final CRC32 check integrated.
    ///
    /// Reads all bytes until EOF and returns an owned vector of them.
    pub async fn read_to_end_crc(mut self) -> Result<Vec<u8>> {
        let mut buffer = Vec::with_capacity(self.entry.uncompressed_size.unwrap().try_into().unwrap());
        self.read_to_end(&mut buffer).await?;

        if self.compare_crc() {
            Ok(buffer)
        } else {
            Err(ZipError::CRC32CheckError)
        }
    }

    /// A convenience method similar to `AsyncReadExt::read_to_string()` but with the final CRC32 check integrated.
    ///
    /// Reads all bytes until EOF and returns an owned string of them.
    pub async fn read_to_string_crc(mut self) -> Result<String> {
        let mut buffer = String::with_capacity(self.entry.uncompressed_size.unwrap().try_into().unwrap());
        self.read_to_string(&mut buffer).await?;

        if self.compare_crc() {
            Ok(buffer)
        } else {
            Err(ZipError::CRC32CheckError)
        }
    }

    /// A convenience method for buffered copying of bytes to a writer with the final CRC32 check integrated.
    ///
    /// # Note
    /// Any bytes written to the writer cannot be unwound, thus the caller should appropriately handle the side effects
    /// of a failed CRC32 check.
    ///
    /// Prefer this method over tokio::io::copy as we have the ability to specify the buffer size (64kb recommended on
    /// modern systems), whereas, tokio's default implementation uses 2kb, so many more calls to read() have to take
    /// place.
    pub async fn copy_to_end_crc<W: AsyncWrite + Unpin>(mut self, writer: &mut W, buffer: usize) -> Result<()> {
        let mut reader = BufReader::with_capacity(buffer, &mut self);
        tokio::io::copy_buf(&mut reader, writer).await.unwrap();

        if self.compare_crc() {
            Ok(())
        } else {
            Err(ZipError::CRC32CheckError)
        }
    }
}

impl<'a, R: AsyncRead + Unpin> AsyncRead for ZipEntryReader<'a, R> {
    fn poll_read(mut self: Pin<&mut Self>, c: &mut Context<'_>, b: &mut ReadBuf<'_>) -> Poll<tokio::io::Result<()>> {
        let prev_len = b.filled().len();
        let poll = Pin::new(&mut self.reader).poll_read(c, b);

        match poll {
            Poll::Ready(Err(_)) | Poll::Pending => return poll,
            _ => {}
        };

        if b.filled().len() - prev_len == 0 {
            self.consumed = true;
        }

        self.hasher.update(&b.filled()[prev_len..b.filled().len()]);
        poll
    }
}

/// A reader which may implement decompression over its inner type, and of which supports owned inner types or mutable
/// borrows of them. Implements identical compression types to that of the crate::spec::compression::Compression enum.
///
/// This underpins entry reading functionality for all three sub-modules (stream, seek, and concurrent).
pub(crate) enum CompressionReader<'a, R: AsyncRead + Unpin> {
    Stored(Take<R>),
    StoredBorrow(Take<&'a mut R>),
    Deflate(DeflateDecoder<BufReader<Take<R>>>),
    DeflateBorrow(DeflateDecoder<BufReader<Take<&'a mut R>>>),
    Bz(BzDecoder<BufReader<Take<R>>>),
    BzBorrow(BzDecoder<BufReader<Take<&'a mut R>>>),
    Lzma(LzmaDecoder<BufReader<Take<R>>>),
    LzmaBorrow(LzmaDecoder<BufReader<Take<&'a mut R>>>),
    Zstd(ZstdDecoder<BufReader<Take<R>>>),
    ZstdBorrow(ZstdDecoder<BufReader<Take<&'a mut R>>>),
    Xz(XzDecoder<BufReader<Take<R>>>),
    XzBorrow(XzDecoder<BufReader<Take<&'a mut R>>>),
}

impl<'a, R: AsyncRead + Unpin> AsyncRead for CompressionReader<'a, R> {
    fn poll_read(mut self: Pin<&mut Self>, c: &mut Context<'_>, b: &mut ReadBuf<'_>) -> Poll<tokio::io::Result<()>> {
        match *self {
            CompressionReader::Stored(ref mut inner) => Pin::new(inner).poll_read(c, b),
            CompressionReader::StoredBorrow(ref mut inner) => Pin::new(inner).poll_read(c, b),
            CompressionReader::Deflate(ref mut inner) => Pin::new(inner).poll_read(c, b),
            CompressionReader::DeflateBorrow(ref mut inner) => Pin::new(inner).poll_read(c, b),
            CompressionReader::Bz(ref mut inner) => Pin::new(inner).poll_read(c, b),
            CompressionReader::BzBorrow(ref mut inner) => Pin::new(inner).poll_read(c, b),
            CompressionReader::Lzma(ref mut inner) => Pin::new(inner).poll_read(c, b),
            CompressionReader::LzmaBorrow(ref mut inner) => Pin::new(inner).poll_read(c, b),
            CompressionReader::Zstd(ref mut inner) => Pin::new(inner).poll_read(c, b),
            CompressionReader::ZstdBorrow(ref mut inner) => Pin::new(inner).poll_read(c, b),
            CompressionReader::Xz(ref mut inner) => Pin::new(inner).poll_read(c, b),
            CompressionReader::XzBorrow(ref mut inner) => Pin::new(inner).poll_read(c, b),
        }
    }
}

impl<'a, R: AsyncRead + Unpin> CompressionReader<'a, R> {
    pub(crate) fn from_reader(compression: &Compression, reader: Take<R>) -> Self {
        match compression {
            Compression::Stored => CompressionReader::Stored(reader),
            Compression::Deflate => CompressionReader::Deflate(DeflateDecoder::new(BufReader::new(reader))),
            Compression::Bz => CompressionReader::Bz(BzDecoder::new(BufReader::new(reader))),
            Compression::Lzma => CompressionReader::Lzma(LzmaDecoder::new(BufReader::new(reader))),
            Compression::Zstd => CompressionReader::Zstd(ZstdDecoder::new(BufReader::new(reader))),
            Compression::Xz => CompressionReader::Xz(XzDecoder::new(BufReader::new(reader))),
        }
    }

    pub(crate) fn from_reader_borrow(compression: &Compression, reader: Take<&'a mut R>) -> Self {
        match compression {
            Compression::Stored => CompressionReader::StoredBorrow(reader),
            Compression::Deflate => CompressionReader::DeflateBorrow(DeflateDecoder::new(BufReader::new(reader))),
            Compression::Bz => CompressionReader::BzBorrow(BzDecoder::new(BufReader::new(reader))),
            Compression::Lzma => CompressionReader::LzmaBorrow(LzmaDecoder::new(BufReader::new(reader))),
            Compression::Zstd => CompressionReader::ZstdBorrow(ZstdDecoder::new(BufReader::new(reader))),
            Compression::Xz => CompressionReader::XzBorrow(XzDecoder::new(BufReader::new(reader))),
        }
    }
}

macro_rules! reader_entry_impl {
    () => {
        /// Returns a shared reference to a list of the ZIP file's entries.
        pub fn entries(&self) -> &Vec<ZipEntry> {
            &self.entries
        }

        /// Searches for an entry with a specific filename.
        pub fn entry(&self, name: &str) -> Option<(usize, &ZipEntry)> {
            for (index, entry) in self.entries().iter().enumerate() {
                if entry.name() == name {
                    return Some((index, entry));
                }
            }
            None
        }
    };
}

pub(crate) use reader_entry_impl;
