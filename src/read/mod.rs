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
use std::borrow::BorrowMut;

use std::convert::TryInto;
use std::pin::Pin;
use std::task::{Context, Poll};

#[cfg(any(feature = "deflate", feature = "bzip2", feature = "zstd", feature = "lzma", feature = "xz"))]
use async_compression::tokio::bufread;
use async_io_utilities::{AsyncDelimiterReader, AsyncPrependReader};
use chrono::{DateTime, Utc};
use crc32fast::Hasher;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, BufReader, ReadBuf, Take};

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
        self.name.ends_with('/')
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
}

pub(crate) enum PrependReader<'a, R: AsyncRead + Unpin> {
    Normal(OwnedReader<'a, R>),
    Prepend(OwnedReader<'a, AsyncPrependReader<R>>),
}

impl<'a, R: AsyncRead + Unpin> AsyncRead for PrependReader<'a, R> {
    fn poll_read(mut self: Pin<&mut Self>, c: &mut Context<'_>, b: &mut ReadBuf<'_>) -> Poll<tokio::io::Result<()>> {
        match *self {
            PrependReader::Normal(ref mut inner) => Pin::new(inner).poll_read(c, b),
            PrependReader::Prepend(ref mut inner) => Pin::new(inner).poll_read(c, b),
        }
    }
}

pub(crate) enum OwnedReader<'a, R: AsyncRead + Unpin> {
    Owned(R),
    Borrow(&'a mut R),
}

impl<'a, R: AsyncRead + Unpin> AsyncRead for OwnedReader<'a, R> {
    fn poll_read(mut self: Pin<&mut Self>, c: &mut Context<'_>, b: &mut ReadBuf<'_>) -> Poll<tokio::io::Result<()>> {
        match *self {
            OwnedReader::Owned(ref mut inner) => Pin::new(inner).poll_read(c, b),
            OwnedReader::Borrow(ref mut inner) => Pin::new(inner).poll_read(c, b),
        }
    }
}

pub(crate) enum LocalReader<'a, R: AsyncRead + Unpin> {
    Standard(CompressionReader<PrependReader<'a, R>>),
    Stream(CompressionReader<AsyncDelimiterReader<PrependReader<'a, R>>>),
}

impl<'a, R: AsyncRead + Unpin> AsyncRead for LocalReader<'a, R> {
    fn poll_read(mut self: Pin<&mut Self>, c: &mut Context<'_>, b: &mut ReadBuf<'_>) -> Poll<tokio::io::Result<()>> {
        match *self {
            LocalReader::Standard(ref mut inner) => Pin::new(inner).poll_read(c, b),
            LocalReader::Stream(ref mut inner) => Pin::new(inner).poll_read(c, b),
        }
    }
}

/// A ZIP file entry reader which may implement decompression.
pub struct ZipEntryReader<'a, R: AsyncRead + Unpin> {
    pub(crate) entry: &'a ZipEntry,
    pub(crate) reader: LocalReader<'a, R>,
    pub(crate) hasher: Hasher,
    pub(crate) consumed: bool,
    pub(crate) state: State,
    pub(crate) data_descriptor: Option<(u32, u32, u32)>,
}

/// The state of the ZIP entry reader.
///
/// The state is expected to go from [`State::ReadData`] to [`State::ReadDescriptor`] and
/// to [`State::PrepareNext`] then back to [`State::ReadData`] **at most** once.
/// It is allowed to never leave the [`State::ReadData`] state, but not allowed to advance
/// its state more than one time, unless the [`ZipEntryReader::poll_data_descriptor`]
/// is adapted to support this.
///
/// This enum is needed to support the [`ZipEntryReader::poll_data_descriptor`] method,
/// `poll*` can be called multiple times and needs a State Machine to behave as intended.
#[derive(Default, Clone, Copy)]
pub(crate) enum State {
    #[default]
    ReadData,
    ReadDescriptor([u8; 12], usize),
    PrepareNext([u8; 12]),
}

impl<'a, R: AsyncRead + Unpin> ZipEntryReader<'a, R> {
    /// Construct an entry reader from its raw parts (a shared reference to the entry and an inner reader).
    pub(crate) fn from_raw(entry: &'a ZipEntry, reader: CompressionReader<PrependReader<'a, R>>, _: bool) -> Self {
        let reader = LocalReader::Standard(reader);
        ZipEntryReader {
            entry,
            reader,
            hasher: Hasher::new(),
            consumed: false,
            state: Default::default(),
            data_descriptor: None,
        }
    }

    /// Construct an entry reader from its raw parts (a shared reference to the entry and an inner reader).
    pub(crate) fn with_data_descriptor(
        entry: &'a ZipEntry,
        reader: CompressionReader<AsyncDelimiterReader<PrependReader<'a, R>>>,
        _: bool,
    ) -> Self {
        let reader = LocalReader::Stream(reader);
        ZipEntryReader {
            entry,
            reader,
            hasher: Hasher::new(),
            consumed: false,
            state: Default::default(),
            data_descriptor: None,
        }
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
        let final_crc = hasher.finalize();

        if self.entry().data_descriptor() {
            self.data_descriptor.expect("Data descriptor was not read").0 == final_crc
        } else {
            self.entry().crc32().expect("Missing CRC32 in Local file header") == final_crc
        }
    }

    /// For Streams, CRC-32, compressed size and uncompressed size may not be known yet (for example,
    /// if the data is being compressed and transferred at the same time).
    ///
    /// This method polls for the **Data Descriptor** values using a State Machine, sets
    /// [`Self::data_descriptor`] and prepares the next entry to be read.
    ///
    /// Note that, this function may fail (with `Poll::Ready(Err(_))`) if the data descriptor is
    /// not present in the entry since it tries to read the 12 bytes corresponding
    /// to **Data descriptor** fields (without the signature).
    ///
    /// The caller must ensure that it only calls this function if the data descriptor is present
    /// (see [`Self::poll_read`] implementation).
    ///
    /// ## Additional notes
    ///
    /// ### Absent Data Descriptor
    /// The data descriptor is not always present in the entry even when the descriptor is not found
    /// in the *Local file header*, but this function is not tolerant for this specific case. If we
    /// plan to skip the descriptor, we should also consider making [`Self::compare_crc`] return `Result`
    /// instead of panicking.
    pub(crate) fn poll_data_descriptor(mut self: Pin<&mut Self>, c: &mut Context<'_>) -> Poll<tokio::io::Result<()>> {
        let state = self.state;

        if let LocalReader::Stream(ref mut inner) = self.borrow_mut().reader {
            if matches!(state, State::ReadData) {
                return Poll::Ready(Ok(()));
            }

            let inner_mut = inner.get_mut();

            let state = if let State::ReadDescriptor(mut descriptor_buf, filled) = state {
                inner_mut.reset();

                let mut buf = ReadBuf::new(&mut descriptor_buf);
                buf.set_filled(filled);
                loop {
                    let rem = buf.remaining();
                    if rem != 0 {
                        let poll = Pin::new(&mut *inner_mut).poll_read(c, &mut buf);
                        match poll {
                            Poll::Ready(Ok(())) => {
                                if buf.remaining() == rem {
                                    return Poll::Ready(Err(std::io::Error::new(
                                        std::io::ErrorKind::UnexpectedEof,
                                        "early eof while reading data descriptor",
                                    )));
                                }
                            }
                            Poll::Pending => {
                                // Update the descriptor buffer. Beware that `State` implements Copy,
                                // and we own the `descriptor_buf`, which means that we are modifying a
                                // copy of it, not the original array, so we really need to manually
                                // update the state descriptor buffer array.
                                let filled = buf.filled().len();
                                self.state = State::ReadDescriptor(descriptor_buf, filled);
                                return Poll::Pending;
                            }
                            _ => return poll,
                        }
                    } else {
                        break;
                    }
                }

                State::PrepareNext(descriptor_buf)
            } else {
                state
            };

            let state = if let State::PrepareNext(descriptor_buf) = state {
                let crc = u32::from_le_bytes(descriptor_buf[0..4].try_into().unwrap());
                let compressed = u32::from_le_bytes(descriptor_buf[4..8].try_into().unwrap());
                let uncompressed = u32::from_le_bytes(descriptor_buf[8..12].try_into().unwrap());

                let mut buffer = Vec::new();
                buffer.extend_from_slice(inner_mut.buffer());

                if let PrependReader::Prepend(inner) = inner_mut.get_mut() {
                    match inner {
                        OwnedReader::Owned(inner) => inner.prepend(&buffer),
                        OwnedReader::Borrow(inner) => inner.prepend(&buffer),
                    };
                }

                self.data_descriptor = Some((crc, compressed, uncompressed));
                State::ReadData
            } else {
                state
            };

            self.state = state;
        }

        Poll::Ready(Ok(()))
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
        return match self.state {
            State::ReadData => {
                let prev_len = b.filled().len();
                let poll = Pin::new(&mut self.reader).poll_read(c, b);

                match poll {
                    Poll::Ready(Err(_)) | Poll::Pending => return poll,
                    _ => {}
                };

                if b.filled().len() - prev_len == 0 {
                    self.consumed = true;

                    if self.data_descriptor.is_none() {
                        self.state = State::ReadDescriptor([0u8; 12], 0);

                        self.poll_data_descriptor(c)
                    } else {
                        poll
                    }
                } else {
                    self.hasher.update(&b.filled()[prev_len..b.filled().len()]);
                    poll
                }
            }
            // Any state other than ReadData means that descriptor is being read.
            _ => self.poll_data_descriptor(c),
        };
    }
}

/// A reader which may implement decompression over its inner type, and of which supports owned inner types or mutable
/// borrows of them. Implements identical compression types to that of the crate::spec::compression::Compression enum.
///
/// This underpins entry reading functionality for all three sub-modules (stream, seek, and concurrent).
pub(crate) enum CompressionReader<R: AsyncRead + Unpin> {
    Stored(Take<R>),
    #[cfg(feature = "deflate")]
    Deflate(bufread::DeflateDecoder<BufReader<Take<R>>>),
    #[cfg(feature = "bzip2")]
    Bz(bufread::BzDecoder<BufReader<Take<R>>>),
    #[cfg(feature = "lzma")]
    Lzma(bufread::LzmaDecoder<BufReader<Take<R>>>),
    #[cfg(feature = "zstd")]
    Zstd(bufread::ZstdDecoder<BufReader<Take<R>>>),
    #[cfg(feature = "xz")]
    Xz(bufread::XzDecoder<BufReader<Take<R>>>),
}

impl<R: AsyncRead + Unpin> CompressionReader<R> {
    pub(crate) fn get_mut(&mut self) -> &mut R {
        match self {
            CompressionReader::Stored(inner) => inner.get_mut(),
            #[cfg(feature = "deflate")]
            CompressionReader::Deflate(inner) => inner.get_mut().get_mut().get_mut(),
            #[cfg(feature = "bzip2")]
            CompressionReader::Bz(inner) => inner.get_mut().get_mut().get_mut(),
            #[cfg(feature = "lzma")]
            CompressionReader::Lzma(inner) => inner.get_mut().get_mut().get_mut(),
            #[cfg(feature = "zstd")]
            CompressionReader::Zstd(inner) => inner.get_mut().get_mut().get_mut(),
            #[cfg(feature = "xz")]
            CompressionReader::Xz(inner) => inner.get_mut().get_mut().get_mut(),
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for CompressionReader<R> {
    fn poll_read(mut self: Pin<&mut Self>, c: &mut Context<'_>, b: &mut ReadBuf<'_>) -> Poll<tokio::io::Result<()>> {
        match *self {
            CompressionReader::Stored(ref mut inner) => Pin::new(inner).poll_read(c, b),
            #[cfg(feature = "deflate")]
            CompressionReader::Deflate(ref mut inner) => Pin::new(inner).poll_read(c, b),
            #[cfg(feature = "bzip2")]
            CompressionReader::Bz(ref mut inner) => Pin::new(inner).poll_read(c, b),
            #[cfg(feature = "lzma")]
            CompressionReader::Lzma(ref mut inner) => Pin::new(inner).poll_read(c, b),
            #[cfg(feature = "zstd")]
            CompressionReader::Zstd(ref mut inner) => Pin::new(inner).poll_read(c, b),
            #[cfg(feature = "xz")]
            CompressionReader::Xz(ref mut inner) => Pin::new(inner).poll_read(c, b),
        }
    }
}

impl<'a, R: AsyncRead + Unpin> CompressionReader<R> {
    pub(crate) fn from_reader(compression: &Compression, reader: Take<R>) -> Self {
        match compression {
            Compression::Stored => CompressionReader::Stored(reader),
            #[cfg(feature = "deflate")]
            Compression::Deflate => CompressionReader::Deflate(bufread::DeflateDecoder::new(BufReader::new(reader))),
            #[cfg(feature = "bzip2")]
            Compression::Bz => CompressionReader::Bz(bufread::BzDecoder::new(BufReader::new(reader))),
            #[cfg(feature = "lzma")]
            Compression::Lzma => CompressionReader::Lzma(bufread::LzmaDecoder::new(BufReader::new(reader))),
            #[cfg(feature = "zstd")]
            Compression::Zstd => CompressionReader::Zstd(bufread::ZstdDecoder::new(BufReader::new(reader))),
            #[cfg(feature = "xz")]
            Compression::Xz => CompressionReader::Xz(bufread::XzDecoder::new(BufReader::new(reader))),
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

        /// Returns an optional ending comment.
        pub fn comment(&self) -> Option<&str> {
            self.comment.as_ref().map(|x| &x[..])
        }
    };
}

pub(crate) use reader_entry_impl;
