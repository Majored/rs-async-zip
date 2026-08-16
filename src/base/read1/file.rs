// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::ready;

use crc32fast::Hasher;
use futures_lite::AsyncReadExt;
use futures_lite::AsyncRead;
use futures_lite::AsyncBufRead;
use futures_lite::io::Take;

use crate::base::read1::ZipOptions;
use crate::error::Result;
use crate::spec::headers1::Compression;
use crate::spec::constructs::LF;
use crate::base::read::io::poll_result_ok;

#[cfg(any(
    feature = "deflate", feature = "bzip2", feature = "zstd",
    feature = "lzma", feature = "xz", feature = "deflate64"
))]
use async_compression::futures::bufread;

/// A reader for a single file in a ZIP archive.
pub struct ZipFileReader<R> {
    reader: CompressedReader<Take<R>>,
    opts: ZipOptions,
    hasher: Hasher,
    read: usize,
    lf: LF,
}

impl<R: AsyncBufRead + Unpin> ZipFileReader<R> {
    pub(crate) fn new(reader: R, lf: LF, opts: ZipOptions) -> Result<Self> {
        // Build the reader stack.
        let reader = reader.take(lf.lfh.compressed_size as u64);
        let reader = CompressedReader::new(reader, lf.lfh.compression)?;

        Ok(Self { reader, hasher: Hasher::default(), read: 0, lf, opts })
    }

    /// Validates the computed CRC32 hash against the expected value.
    /// 
    /// Once all data has been consumed (ie. reached EOF), call this method.
    pub fn validate(&mut self) -> Result<()> {
        if self.opts.validate_crc_match_against_read {
            let computed_hash = std::mem::take(&mut self.hasher).finalize();

            if computed_hash != self.lf.lfh.crc {
                return Err(crate::error::ZipError::CRC32CheckError);
            }
        }

        if self.opts.validate_uncompressed_size_match_against_read {
            if self.read as u64 != self.lf.lfh.uncompressed_size.into() {
                return Err(crate::error::ZipError::InvalidUncompressedSizeHeaderMatch); // TODO: new error
            }
        }

        // TODO: compressed size validation.

        Ok(())
    }
}

impl<R: AsyncBufRead + Unpin> AsyncRead for ZipFileReader<R> {
    fn poll_read(mut self: Pin<&mut Self>,cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        let written = Pin::new(&mut self.reader).poll_read(cx, buf);
        let written = poll_result_ok!(ready!(written));
        self.read += written;

        if let Some(max) = &self.opts.max_uncompressed_size_per_file {
            if self.read as u64 > *max {
                return Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, "Max uncompressed size exceeded")));
            }
        }

        Pin::new(&mut self.hasher).update(&buf[..written]);
        Poll::Ready(Ok(written))
    }
}

pub(crate) enum CompressedReader<R> {
    Stored(R),
    #[cfg(feature = "deflate")]
    Deflated(bufread::DeflateDecoder<R>),
    #[cfg(feature = "deflate64")]
    Deflate64(bufread::Deflate64Decoder<R>),
    #[cfg(feature = "bzip2")]
    Bz(bufread::BzDecoder<R>),
    #[cfg(feature = "lzma")]
    Lzma(bufread::LzmaDecoder<R>),
    #[cfg(feature = "zstd")]
    Zstd(bufread::ZstdDecoder<R>),
    #[cfg(feature = "xz")]
    Xz(bufread::XzDecoder<R>),
}

impl<R: AsyncBufRead + Unpin> CompressedReader<R> {
    pub(crate) fn new(reader: R, compression: Compression) -> Result<Self> {
        match compression {
            Compression::Stored => Ok(Self::Stored(reader)),
            #[cfg(feature = "deflate")]
            Compression::Deflate => Ok(Self::Deflated(bufread::DeflateDecoder::new(reader))),
            #[cfg(feature = "deflate64")]
            Compression::Deflate64 => Ok(Self::Deflate64(bufread::Deflate64Decoder::new(reader))),
            #[cfg(feature = "bzip2")]
            Compression::Bz => Ok(Self::Bz(bufread::BzDecoder::new(reader))),
            #[cfg(feature = "lzma")]
            Compression::Lzma => Ok(Self::Lzma(bufread::LzmaDecoder::new(reader))),
            #[cfg(feature = "zstd")]
            Compression::Zstd => Ok(Self::Zstd(bufread::ZstdDecoder::new(reader))),
            #[cfg(feature = "xz")]
            Compression::Xz => Ok(Self::Xz(bufread::XzDecoder::new(reader))),
            _ => Err(crate::error::ZipError::CompressionNotSupported(compression as u16)),
        }
    }
}

impl<R: AsyncBufRead + Unpin> AsyncRead for CompressedReader<R> {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        match &mut *self {
            CompressedReader::Stored(reader) => Pin::new(reader).poll_read(cx, buf),
            #[cfg(feature = "deflate")]
            CompressedReader::Deflated(reader) => Pin::new(reader).poll_read(cx, buf),
            #[cfg(feature = "deflate64")]
            CompressedReader::Deflate64(reader) => Pin::new(reader).poll_read(cx, buf),
            #[cfg(feature = "bzip2")]
            CompressedReader::Bz(reader) => Pin::new(reader).poll_read(cx, buf),
            #[cfg(feature = "lzma")]
            CompressedReader::Lzma(reader) => Pin::new(reader).poll_read(cx, buf),
            #[cfg(feature = "zstd")]
            CompressedReader::Zstd(reader) => Pin::new(reader).poll_read(cx, buf),
            #[cfg(feature = "xz")]
            CompressedReader::Xz(reader) => Pin::new(reader).poll_read(cx, buf),
        }
    }
}
