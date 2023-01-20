// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::spec::compression::Compression;

use std::pin::Pin;
use std::task::{Context, Poll};

#[cfg(any(feature = "deflate", feature = "bzip2", feature = "zstd", feature = "lzma", feature = "xz"))]
use async_compression::tokio::bufread;

use pin_project::pin_project;
use tokio::io::{AsyncBufRead, AsyncRead, ReadBuf};

/// A wrapping reader which holds concrete types for all respective compression method readers.
#[pin_project(project = CompressedReaderProj)]
pub(crate) enum CompressedReader<R> {
    Stored(#[pin] R),
    #[cfg(feature = "deflate")]
    Deflate(#[pin] bufread::DeflateDecoder<R>),
    #[cfg(feature = "bzip2")]
    Bz(#[pin] bufread::BzDecoder<R>),
    #[cfg(feature = "lzma")]
    Lzma(#[pin] bufread::LzmaDecoder<R>),
    #[cfg(feature = "zstd")]
    Zstd(#[pin] bufread::ZstdDecoder<R>),
    #[cfg(feature = "xz")]
    Xz(#[pin] bufread::XzDecoder<R>),
}

impl<R> CompressedReader<R>
where
    R: AsyncBufRead + Unpin,
{
    /// Constructs a new wrapping reader from a generic [`AsyncBufRead`] implementer.
    pub(crate) fn new(reader: R, compression: Compression) -> Self {
        match compression {
            Compression::Stored => CompressedReader::Stored(reader),
            #[cfg(feature = "deflate")]
            Compression::Deflate => CompressedReader::Deflate(bufread::DeflateDecoder::new(reader)),
            #[cfg(feature = "bzip2")]
            Compression::Bz => CompressedReader::Bz(bufread::BzDecoder::new(reader)),
            #[cfg(feature = "lzma")]
            Compression::Lzma => CompressedReader::Lzma(bufread::LzmaDecoder::new(reader)),
            #[cfg(feature = "zstd")]
            Compression::Zstd => CompressedReader::Zstd(bufread::ZstdDecoder::new(reader)),
            #[cfg(feature = "xz")]
            Compression::Xz => CompressedReader::Xz(bufread::XzDecoder::new(reader)),
        }
    }

    /// Consumes this reader and returns the inner value.
    pub(crate) fn into_inner(self) -> R {
        match self {
            CompressedReader::Stored(inner) => inner,
            #[cfg(feature = "deflate")]
            CompressedReader::Deflate(inner) => inner.into_inner(),
            #[cfg(feature = "bzip2")]
            CompressedReader::Bz(inner) => inner.into_inner(),
            #[cfg(feature = "lzma")]
            CompressedReader::Lzma(inner) => inner.into_inner(),
            #[cfg(feature = "zstd")]
            CompressedReader::Zstd(inner) => inner.into_inner(),
            #[cfg(feature = "xz")]
            CompressedReader::Xz(inner) => inner.into_inner(),
        }
    }
}

impl<R> AsyncRead for CompressedReader<R>
where
    R: AsyncBufRead + Unpin,
{
    fn poll_read(self: Pin<&mut Self>, c: &mut Context<'_>, b: &mut ReadBuf<'_>) -> Poll<tokio::io::Result<()>> {
        match self.project() {
            CompressedReaderProj::Stored(inner) => inner.poll_read(c, b),
            #[cfg(feature = "deflate")]
            CompressedReaderProj::Deflate(inner) => inner.poll_read(c, b),
            #[cfg(feature = "bzip2")]
            CompressedReaderProj::Bz(inner) => inner.poll_read(c, b),
            #[cfg(feature = "lzma")]
            CompressedReaderProj::Lzma(inner) => inner.poll_read(c, b),
            #[cfg(feature = "zstd")]
            CompressedReaderProj::Zstd(inner) => inner.poll_read(c, b),
            #[cfg(feature = "xz")]
            CompressedReaderProj::Xz(inner) => inner.poll_read(c, b),
        }
    }
}
