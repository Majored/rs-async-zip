// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module which supports stream reading ZIP files.
//! 
//! # Note
//! 
//! 
//! # Example
//! ```
//! let mut reader = BufReader::new(File::open("Archive.zip").await.unwrap());
//! let mut zip_stream = ZipStreamReader::new(&mut reader).unwrap();
//!
//! loop {
//!     let mut entry = zip_stream.next_entry().await.unwrap().unwrap();
//!     println!("Name: {}", entry.file_name());
//!     entry.consume().await.unwrap();
//! }
//! ```

use crate::error::ZipError;
use crate::error::Result;

use std::marker::{Send, Unpin};
use std::pin::Pin;
use std::task::{Context, Poll};

use async_compression::tokio::bufread::{DeflateDecoder, BzDecoder, LzmaDecoder, ZstdDecoder, XzDecoder};
use tokio::io::{AsyncBufRead, AsyncRead, AsyncReadExt, ReadBuf, Take};

/// A type accepted as input to ZipStreamReader.
pub(crate) type AsyncReader = dyn AsyncBufRead + Unpin + Send;

/// An enum of possible concrete compression decoders which are supported by this crate.
pub(crate) enum CompressionReader<'a> {
    Stored(Take<&'a mut AsyncReader>),
    Deflate(DeflateDecoder<Take<&'a mut AsyncReader>>),
    Bz(BzDecoder<Take<&'a mut AsyncReader>>),
    Lzma(LzmaDecoder<Take<&'a mut AsyncReader>>),
    Zstd(ZstdDecoder<Take<&'a mut AsyncReader>>),
    Xz(XzDecoder<Take<&'a mut AsyncReader>>),
}

pub struct ZipStreamReader<'a> {
    reader: &'a mut AsyncReader,
}

pub struct ZipStreamFile<'a> {
    file_name: String,
    compressed_size: u32,
    uncompressed_size: u32,
    crc: u32,
    extra: Vec<u8>,
    bytes_read: u32,
    reader: CompressionReader<'a>,
}

impl<'a> AsyncRead for ZipStreamFile<'a> {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<tokio::io::Result<()>> {
        let size_before = buf.filled().len();
        let poll = match self.reader {
            CompressionReader::Stored(ref mut inner) => Pin::new(inner).poll_read(cx, buf),
            CompressionReader::Deflate(ref mut inner) => Pin::new(inner).poll_read(cx, buf),
            CompressionReader::Bz(ref mut inner) => Pin::new(inner).poll_read(cx, buf),
            CompressionReader::Lzma(ref mut inner) => Pin::new(inner).poll_read(cx, buf),
            CompressionReader::Zstd(ref mut inner) => Pin::new(inner).poll_read(cx, buf),
            CompressionReader::Xz(ref mut inner) => Pin::new(inner).poll_read(cx, buf),
        };
        self.bytes_read += (buf.filled().len() - size_before) as u32;
        poll
    }
}

impl<'a> ZipStreamFile<'a> {
    /// Returns a reference to the file's name.
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// Returns the file's compressed size in bytes.
    pub fn compressed_size(&self) -> u32 {
        self.compressed_size
    }

    /// Returns the file's uncompressed size in bytes.
    pub fn uncompressed_size(&self) -> u32 {
        self.uncompressed_size
    }

    /// Returns the file's cyclic redundancy check (CRC) value.
    pub fn crc(&self) -> u32 {
        self.crc
    }

    /// Returns a reference to the file's extra field data.
    pub fn extra(&self) -> &Vec<u8> {
        &self.extra
    }

    /// Returns whether or not the file has been fully read.
    pub fn is_eof(&self) -> bool {
        self.uncompressed_size == self.bytes_read
    }

    /// Returns whether or not the file is a directory.
    pub fn is_dir(&self) -> bool {
        self.file_name.ends_with("/") && self.uncompressed_size == 0 && self.compressed_size == 0
    }

    /// Consumes all bytes within this file.
    ///
    /// Any file's contents will need to be fully read before a call to ZIPStreamReader::next_entry() is made so that
    /// the inner reader is already positioned at the start of the local file header deliminator. If you don't want to
    /// fully read the file content's yourself, this method can be called to consume the bytes for you before dropping.
    pub async fn consume(&mut self) -> Result<()> {
        let mut buffer = vec![0; 8192];

        loop {
            match self.read(&mut buffer).await {
                Ok(read) => {
                    if read == 0 {
                        break;
                    }
                }
                Err(_) => return Err(ZipError::ReadFailed),
            };
        }

        Ok(())
    }
}

impl<'a> ZipStreamReader<'a> {
    /// Constructs a new instance from a mutable reference to a buffered reader.
    pub fn new(reader: &'a mut AsyncReader) -> Result<ZipStreamReader<'a>> {
        Ok(ZipStreamReader { reader })
    }

    /// Returns the next file in the archive.
    /// 
    /// # Note
    /// This function requries the reader already be placed at the start of the next local file header. Ensure that any
    /// previous files constrcuted before this call have fully read their data. This can be done by calling
    /// ZipStreamFile::consume().
    /// 
    /// # Example
    /// ```
    /// loop {
    ///     let entry_opt = match zip_stream.next_entry().await {
    ///         Ok(entry) => entry,
    ///         Err(_) => break,
    ///     };
    /// 
    ///     match entry_opt {
    ///         Some(entry) => println!("Name: {}", entry.file_name()),
    ///         None => break,
    ///     };
    /// }
    /// ```
    pub async fn next_entry<'b>(&'b mut self) -> Result<Option<ZipStreamFile<'b>>> {
        let flhd = read_u32(self.reader).await?;

        match flhd {
            crate::delim::LFHD => (),
            crate::delim::CDFHD => return Ok(None),
            _ => return Err(ZipError::LocalFileHeaderError(flhd)),
        };

        let version = read_u16(self.reader).await?;
        let flags = read_u16(self.reader).await?;
        let compression = read_u16(self.reader).await?;
        let mod_time = read_u16(self.reader).await?;
        let mod_date = read_u16(self.reader).await?;
        let crc = read_u32(self.reader).await?;
        let compressed_size = read_u32(self.reader).await?;
        let uncompressed_size = read_u32(self.reader).await?;
        let file_name_length = read_u16(self.reader).await?;
        let extra_field_length = read_u16(self.reader).await?;

        let file_name = read_string(self.reader, file_name_length).await?;
        let extra = read(self.reader, extra_field_length).await?;

        let limit_reader = self.reader.take(compressed_size.into());
        let file_reader = match compression {
            0 => CompressionReader::Stored(limit_reader),
            8 => CompressionReader::Deflate(DeflateDecoder::new(limit_reader)),
            12 => CompressionReader::Bz(BzDecoder::new(limit_reader)),
            14 => CompressionReader::Lzma(LzmaDecoder::new(limit_reader)),
            93 => CompressionReader::Zstd(ZstdDecoder::new(limit_reader)),
            95 => CompressionReader::Xz(XzDecoder::new(limit_reader)),
            _ => return Err(ZipError::UnsupportedCompressionError(compression)),
        };

        let zip_file = ZipStreamFile {
            file_name,
            compressed_size,
            uncompressed_size,
            crc,
            extra,
            bytes_read: 0,
            reader: file_reader,
        };

        Ok(Some(zip_file))
    }
}

async fn read(reader: &mut AsyncReader, length: u16) -> Result<Vec<u8>> {
    let mut buffer = vec![0; length as usize];
    reader.read(&mut buffer).await.map_err(|_| ZipError::ReadFailed)?;
    Ok(buffer)
}

async fn read_string(reader: &mut AsyncReader, length: u16) -> Result<String> {
    let mut buffer = String::with_capacity(length as usize);
    reader
        .take(length as u64)
        .read_to_string(&mut buffer)
        .await
        .map_err(|_| ZipError::ReadFailed)?;
    Ok(buffer)
}

async fn read_u32(reader: &mut AsyncReader) -> Result<u32> {
    Ok(reader.read_u32_le().await.map_err(|_| ZipError::ReadFailed)?)
}

async fn read_u16(reader: &mut AsyncReader) -> Result<u16> {
    Ok(reader.read_u16_le().await.map_err(|_| ZipError::ReadFailed)?)
}
