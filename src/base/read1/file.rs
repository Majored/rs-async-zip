// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures_lite::AsyncReadExt;
use futures_lite::AsyncRead;
use futures_lite::AsyncBufRead;
use futures_lite::io::Take;

use crate::error::Result;
use crate::spec::headers1::Compression;
use crate::spec::headers1::LF;

#[cfg(any(
    feature = "deflate", feature = "bzip2", feature = "zstd",
    feature = "lzma", feature = "xz", feature = "deflate64"
))]
use async_compression::futures::bufread;

pub struct ZipFileReader<R> {
    reader: CompressedReader<Take<R>>,
    lf: LF,
}

impl<R: AsyncBufRead + Unpin> ZipFileReader<R> {
    pub(crate) fn new(reader: R, lf: LF) -> Result<Self> {
        let reader = reader.take(lf.lfh.compressed_size as u64);
        let reader = CompressedReader::new(reader, lf.lfh.compression)?;

        Ok(Self { reader, lf })
    }
}

impl<R: AsyncBufRead + Unpin> AsyncRead for ZipFileReader<R> {
    fn poll_read(mut self: Pin<&mut Self>,cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.reader).poll_read(cx, buf)
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
