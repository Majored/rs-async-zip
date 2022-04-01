// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::spec::compression::Compression;

use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_compression::tokio::write::{BzEncoder, DeflateEncoder, LzmaEncoder, XzEncoder, ZstdEncoder};
use async_io_utilities::AsyncOffsetWriter;
use tokio::io::AsyncWrite;

pub enum CompressedAsyncWriter<'b, W: AsyncWrite + Unpin> {
    Stored(ShutdownIgnoredWriter<&'b mut AsyncOffsetWriter<W>>),
    Deflate(DeflateEncoder<ShutdownIgnoredWriter<&'b mut AsyncOffsetWriter<W>>>),
    Bz(BzEncoder<ShutdownIgnoredWriter<&'b mut AsyncOffsetWriter<W>>>),
    Lzma(LzmaEncoder<ShutdownIgnoredWriter<&'b mut AsyncOffsetWriter<W>>>),
    Zstd(ZstdEncoder<ShutdownIgnoredWriter<&'b mut AsyncOffsetWriter<W>>>),
    Xz(XzEncoder<ShutdownIgnoredWriter<&'b mut AsyncOffsetWriter<W>>>),
}

impl<'b, W: AsyncWrite + Unpin> CompressedAsyncWriter<'b, W> {
    pub fn from_raw(writer: &'b mut AsyncOffsetWriter<W>, compression: Compression) -> Self {
        match compression {
            Compression::Stored => CompressedAsyncWriter::Stored(ShutdownIgnoredWriter {0: writer}),
            Compression::Deflate => CompressedAsyncWriter::Deflate(DeflateEncoder::new(ShutdownIgnoredWriter {0: writer})),
            Compression::Bz => CompressedAsyncWriter::Bz(BzEncoder::new(ShutdownIgnoredWriter {0: writer})),
            Compression::Lzma => CompressedAsyncWriter::Lzma(LzmaEncoder::new(ShutdownIgnoredWriter {0: writer})),
            Compression::Zstd => CompressedAsyncWriter::Zstd(ZstdEncoder::new(ShutdownIgnoredWriter {0: writer})),
            Compression::Xz => CompressedAsyncWriter::Xz(XzEncoder::new(ShutdownIgnoredWriter {0: writer})),
        }
    }

    pub fn into_inner(self) -> &'b mut AsyncOffsetWriter<W> {
        match self {
            CompressedAsyncWriter::Stored(inner) => inner.into_inner(),
            CompressedAsyncWriter::Deflate(inner) => inner.into_inner().into_inner(),
            CompressedAsyncWriter::Bz(inner) => inner.into_inner().into_inner(),
            CompressedAsyncWriter::Lzma(inner) => inner.into_inner().into_inner(),
            CompressedAsyncWriter::Zstd(inner) => inner.into_inner().into_inner(),
            CompressedAsyncWriter::Xz(inner) => inner.into_inner().into_inner(),
        }
    }
}

impl<'b, W: AsyncWrite + Unpin> AsyncWrite for CompressedAsyncWriter<'b, W> {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<std::result::Result<usize, Error>> {
        match *self {
            CompressedAsyncWriter::Stored(ref mut inner) => Pin::new(inner).poll_write(cx, buf),
            CompressedAsyncWriter::Deflate(ref mut inner) => Pin::new(inner).poll_write(cx, buf),
            CompressedAsyncWriter::Bz(ref mut inner) => Pin::new(inner).poll_write(cx, buf),
            CompressedAsyncWriter::Lzma(ref mut inner) => Pin::new(inner).poll_write(cx, buf),
            CompressedAsyncWriter::Zstd(ref mut inner) => Pin::new(inner).poll_write(cx, buf),
            CompressedAsyncWriter::Xz(ref mut inner) => Pin::new(inner).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<std::result::Result<(), Error>> {
        match *self {
            CompressedAsyncWriter::Stored(ref mut inner) => Pin::new(inner).poll_flush(cx),
            CompressedAsyncWriter::Deflate(ref mut inner) => Pin::new(inner).poll_flush(cx),
            CompressedAsyncWriter::Bz(ref mut inner) => Pin::new(inner).poll_flush(cx),
            CompressedAsyncWriter::Lzma(ref mut inner) => Pin::new(inner).poll_flush(cx),
            CompressedAsyncWriter::Zstd(ref mut inner) => Pin::new(inner).poll_flush(cx),
            CompressedAsyncWriter::Xz(ref mut inner) => Pin::new(inner).poll_flush(cx),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<std::result::Result<(), Error>> {
        match *self {
            CompressedAsyncWriter::Stored(ref mut inner) => Pin::new(inner).poll_shutdown(cx),
            CompressedAsyncWriter::Deflate(ref mut inner) => Pin::new(inner).poll_shutdown(cx),
            CompressedAsyncWriter::Bz(ref mut inner) => Pin::new(inner).poll_shutdown(cx),
            CompressedAsyncWriter::Lzma(ref mut inner) => Pin::new(inner).poll_shutdown(cx),
            CompressedAsyncWriter::Zstd(ref mut inner) => Pin::new(inner).poll_shutdown(cx),
            CompressedAsyncWriter::Xz(ref mut inner) => Pin::new(inner).poll_shutdown(cx),
        }
    }
}

pub struct ShutdownIgnoredWriter<W: AsyncWrite + Unpin>(W);

impl<W: AsyncWrite + Unpin> ShutdownIgnoredWriter<W> {
    pub fn into_inner(self) -> W {
        self.0
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for ShutdownIgnoredWriter<W> {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<std::result::Result<usize, Error>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<std::result::Result<(), Error>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context) -> Poll<std::result::Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}
