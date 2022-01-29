// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use tokio::io::{AsyncWrite};
use crate::Compression;

use std::io::Error;
use std::pin::Pin;
use std::task::{Poll, Context};

use async_compression::tokio::write::{BzEncoder, DeflateEncoder, LzmaEncoder, XzEncoder, ZstdEncoder};

pub enum CompressedAsyncWriter<'a, W: AsyncWrite + Unpin> {
    Deflate(DeflateEncoder<&'a mut W>),
    Bz(BzEncoder<&'a mut W>),
    Lzma(LzmaEncoder<&'a mut W>),
    Zstd(ZstdEncoder<&'a mut W>),
    Xz(XzEncoder<&'a mut W>),
}

impl<'a, W: AsyncWrite + Unpin> CompressedAsyncWriter<'a, W> {
    pub fn from_raw(writer: &'a mut W, compression: Compression) -> Self {
        match compression {
            Compression::Stored => unreachable!(),
            Compression::Deflate => CompressedAsyncWriter::Deflate(DeflateEncoder::new(writer)),
            Compression::Bz => CompressedAsyncWriter::Bz(BzEncoder::new(writer)),
            Compression::Lzma => CompressedAsyncWriter::Lzma(LzmaEncoder::new(writer)),
            Compression::Zstd => CompressedAsyncWriter::Zstd(ZstdEncoder::new(writer)),
            Compression::Xz => CompressedAsyncWriter::Xz(XzEncoder::new(writer)),
        }
    }
}

impl<'a, 'brw, W: AsyncWrite + Unpin> AsyncWrite for CompressedAsyncWriter<'a, W> {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<std::result::Result<usize, Error>> {
        match *self {
            CompressedAsyncWriter::Deflate(ref mut inner) => Pin::new(inner).poll_write(cx, buf),
            CompressedAsyncWriter::Bz(ref mut inner) => Pin::new(inner).poll_write(cx, buf),
            CompressedAsyncWriter::Lzma(ref mut inner) => Pin::new(inner).poll_write(cx, buf),
            CompressedAsyncWriter::Zstd(ref mut inner) => Pin::new(inner).poll_write(cx, buf),
            CompressedAsyncWriter::Xz(ref mut inner) => Pin::new(inner).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<std::result::Result<(), Error>> {
        match *self {
            CompressedAsyncWriter::Deflate(ref mut inner) => Pin::new(inner).poll_flush(cx),
            CompressedAsyncWriter::Bz(ref mut inner) => Pin::new(inner).poll_flush(cx),
            CompressedAsyncWriter::Lzma(ref mut inner) => Pin::new(inner).poll_flush(cx),
            CompressedAsyncWriter::Zstd(ref mut inner) => Pin::new(inner).poll_flush(cx),
            CompressedAsyncWriter::Xz(ref mut inner) => Pin::new(inner).poll_flush(cx),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<std::result::Result<(), Error>> {
        match *self {
            CompressedAsyncWriter::Deflate(ref mut inner) => Pin::new(inner).poll_shutdown(cx),
            CompressedAsyncWriter::Bz(ref mut inner) => Pin::new(inner).poll_shutdown(cx),
            CompressedAsyncWriter::Lzma(ref mut inner) => Pin::new(inner).poll_shutdown(cx),
            CompressedAsyncWriter::Zstd(ref mut inner) => Pin::new(inner).poll_shutdown(cx),
            CompressedAsyncWriter::Xz(ref mut inner) => Pin::new(inner).poll_shutdown(cx),
        }
    }
}
