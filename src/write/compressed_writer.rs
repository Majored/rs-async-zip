// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::write::offset_writer::OffsetAsyncWriter;
use crate::Compression;

use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_compression::tokio::write::{BzEncoder, DeflateEncoder, LzmaEncoder, XzEncoder, ZstdEncoder};
use tokio::io::AsyncWrite;

pub enum CompressedAsyncWriter<'b, W: AsyncWrite + Unpin> {
    Stored(&'b mut OffsetAsyncWriter<W>),
    Deflate(DeflateEncoder<&'b mut OffsetAsyncWriter<W>>),
    Bz(BzEncoder<&'b mut OffsetAsyncWriter<W>>),
    Lzma(LzmaEncoder<&'b mut OffsetAsyncWriter<W>>),
    Zstd(ZstdEncoder<&'b mut OffsetAsyncWriter<W>>),
    Xz(XzEncoder<&'b mut OffsetAsyncWriter<W>>),
}

impl<'b, W: AsyncWrite + Unpin> CompressedAsyncWriter<'b, W> {
    pub fn from_raw(writer: &'b mut OffsetAsyncWriter<W>, compression: Compression) -> Self {
        match compression {
            Compression::Stored => CompressedAsyncWriter::Stored(writer),
            Compression::Deflate => CompressedAsyncWriter::Deflate(DeflateEncoder::new(writer)),
            Compression::Bz => CompressedAsyncWriter::Bz(BzEncoder::new(writer)),
            Compression::Lzma => CompressedAsyncWriter::Lzma(LzmaEncoder::new(writer)),
            Compression::Zstd => CompressedAsyncWriter::Zstd(ZstdEncoder::new(writer)),
            Compression::Xz => CompressedAsyncWriter::Xz(XzEncoder::new(writer)),
        }
    }

    pub fn into_inner(self) -> &'b mut OffsetAsyncWriter<W> {
        match self {
            CompressedAsyncWriter::Stored(inner) => inner,
            CompressedAsyncWriter::Deflate(inner) => inner.into_inner(),
            CompressedAsyncWriter::Bz(inner) => inner.into_inner(),
            CompressedAsyncWriter::Lzma(inner) => inner.into_inner(),
            CompressedAsyncWriter::Zstd(inner) => inner.into_inner(),
            CompressedAsyncWriter::Xz(inner) => inner.into_inner(),
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
